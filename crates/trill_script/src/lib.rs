mod error;
mod lexer;
mod parser;

use std::fmt::Debug;

use codespan_reporting::files::SimpleFiles;
use error::Location;
use error::ScriptReport;
use parser::Definition;
use parser::Parser;
use ustr::Ustr;
use ustr::UstrMap;

use trill_core::ResponseEngineCompiler;
use trill_core::engine::ResponseEngine;

#[derive(Debug, Default)]
pub struct ScriptCompiler {
    partition_variables: Vec<Ustr>,
    files: SimpleFiles<Ustr, String>,
}

impl ScriptCompiler {
    pub fn new() -> ScriptCompiler {
        ScriptCompiler::default()
    }

    pub fn add_module(&mut self, name: impl Into<Ustr>, source: impl ToString) {
        self.files.add(name.into(), source.to_string());
    }

    pub fn with_module(mut self, name: impl Into<Ustr>, source: impl ToString) -> Self {
        self.add_module(name, source);
        self
    }

    pub fn add_partition_variable(&mut self, variable: impl Into<Ustr>) {
        self.partition_variables.push(variable.into());
    }

    pub fn with_partition_variable(mut self, variable: impl Into<Ustr>) -> Self {
        self.add_partition_variable(variable);
        self
    }

    pub fn compile(self) -> (Option<ResponseEngine>, ScriptReport) {
        // First parse all the sources
        let mut compiler = ResponseEngineCompiler::new();
        let mut parse_errors = Vec::default();

        let mut criterion_locations = UstrMap::default();
        let mut rule_locations = UstrMap::default();
        let mut response_group_locations = UstrMap::default();

        let mut i = 0;
        while let Ok(file) = self.files.get(i) {
            let mut parser = Parser::new(file.source());
            loop {
                match parser.maybe_parse_definition() {
                    Ok(None) => break,
                    Ok(Some((Definition::Criterion { name, criterion }, span))) => {
                        criterion_locations.insert(name, Location { file_id: i, span });
                        compiler.with_criterion(name, criterion);
                    }
                    Ok(Some((Definition::Rule { name, rule }, span))) => {
                        rule_locations.insert(name, Location { file_id: i, span });
                        compiler.with_rule(name, rule);
                    }
                    Ok(Some((
                        Definition::ResponseGroup {
                            name,
                            response_group,
                        },
                        span,
                    ))) => {
                        response_group_locations.insert(name, Location { file_id: i, span });
                        compiler.with_response_group(name, response_group);
                    }
                    Err(error) => {
                        parse_errors.push((i, error));
                        break;
                    }
                }
            }
            i += 1;
        }

        let mut report = ScriptReport {
            compile_errors: Vec::new(),
            parse_errors,
            files: self.files,
            criterion_locations,
            rule_locations,
            response_group_locations,
        };

        if !report.parse_errors.is_empty() {
            return (None, report);
        }

        for var in self.partition_variables {
            compiler.with_partition_variable(var);
        }

        let (engine, compiler_report) = compiler.finish();
        report.compile_errors = compiler_report.errors;

        (engine, report)
    }
}

#[cfg(test)]
mod test {
    use trill_core::engine::StatementSet;
    use ustr::Ustr;

    use crate::ScriptCompiler;

    #[test]
    fn compile_criterion_numeric_equals() {
        let script = "(criterion Name (variable == 0.0))";
        let (engine, report) = ScriptCompiler::new()
            .with_module("script.trl", script)
            .compile();

        report.print();

        assert!(engine.is_some());
    }

    #[test]
    fn compile_rule_response_group() {
        let script = r#"
            (rule RuleName () (GroupName))
            (response GroupName (line "test"))
        "#;
        let (engine, report) = ScriptCompiler::new()
            .with_module("script.trl", script)
            .compile();

        report.print();

        assert!(engine.is_some());
    }

    #[test]
    fn compile_script() {
        let script = r#"
            (criterion PlayerNear (distance_to_player in 0..500))
            (criterion ConceptTalkStare (concept == talk_stare) weight 5)
            (criterion IsCitizen (class_name == citizen))
            (criterion IsMiles (target_name == miles))
            (criterion NPCIdle (npc_state == idle))

            (rule CitizenTalkStare (ConceptTalkStare IsCitizen NPCIdle) (CitizenTalkStare))
            (rule MilesTalkStare (ConceptTalkStare IsCitizen NPCIdle IsMiles) (MilesTalkStare))

            (response MilesTalkStare
                (line "Oh hi! I'm Miles"))

            (response CitizenTalkStare shuffle
                (line "What are you looking at, punk?")
                (line "Hey you, get going.")
                (line "What you staring at me for?")
                (line "Do I know you?")
                (line "You waiting for somebody or something?"))
        "#;

        let (engine, report) = ScriptCompiler::new()
            .with_partition_variable("concept")
            .with_module("script.trl", script)
            .compile();

        report.print();

        let mut engine = engine.unwrap();

        let actor = StatementSet::new()
            .with("distance_to_player", 20.0)
            .with("class_name", "citizen")
            .with("target_name", "miles")
            .with("npc_state", "idle");

        let query = StatementSet::new().with("concept", "talk_stare");

        let query = [&actor, &query];
        let mut rng = rand::rng();
        let resp = engine.find_best_response(query, &mut rng).unwrap();

        let line = resp.get(&Ustr::from("line")).unwrap();

        assert_eq!(line, "Oh hi! I'm Miles");
    }
}
