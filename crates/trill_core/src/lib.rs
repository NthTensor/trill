pub mod engine;

use core::fmt;
use std::collections::HashMap;

use engine::Encoder;
use ustr::Ustr;

use engine::EngineCriterion;
use engine::EngineResponseGroup;
use engine::EngineRule;
use engine::ResponseDispatcher;
use engine::ResponseEngine;
use engine::RulePartitions;
use ustr::UstrMap;
use ustr::UstrSet;

#[derive(Debug)]
pub struct Criterion {
    pub variable: Ustr,
    pub predicate: Predicate,
    pub weight: f32,
}

#[derive(Debug)]
pub enum Predicate {
    BoolEqual(bool),
    NumEqual(f32),
    NumRange(Option<f32>, Option<f32>),
    StrEqual(Ustr),
}

impl Criterion {
    fn build(self, name: Ustr, ctx: &mut Context) -> EngineCriterion {
        // Generate some rudimentary type info
        let infered_type = match self.predicate {
            Predicate::BoolEqual(_) => Type::Bool,
            Predicate::NumEqual(_) | Predicate::NumRange(_, _) => Type::Num,
            Predicate::StrEqual(_) => Type::Str,
        };
        let usage = VariableUsage {
            infered_type,
            location: VariableLocation::Criterion(name),
        };
        if let Some(variable_usages) = ctx.variable_usages.get_mut(&self.variable) {
            variable_usages.push(usage);
        } else {
            ctx.variable_usages.insert(self.variable, vec![usage]);
        }

        // Finalize
        let (min, max) = match self.predicate {
            crate::Predicate::BoolEqual(false) => (0.0, 0.0),
            crate::Predicate::BoolEqual(true) => (1.0, 1.0),
            crate::Predicate::NumEqual(num) => (num, num),
            crate::Predicate::NumRange(min, max) => (
                min.unwrap_or(f32::NEG_INFINITY),
                max.unwrap_or(f32::INFINITY),
            ),
            crate::Predicate::StrEqual(ustr) => {
                let encoding = ctx.encoder.encode_ustr(ustr);
                (encoding, encoding)
            }
        };
        EngineCriterion {
            variable: self.variable,
            min,
            max,
        }
    }
}

#[derive(Debug)]
pub struct Rule {
    pub criteria: Vec<Ustr>,
    pub response_groups: Vec<Ustr>,
    pub instructions: Vec<Instruction>,
}

#[derive(Debug)]
pub struct Instruction {
    pub variable: Ustr,
    pub global: bool,
    pub operation: Operation,
}

#[derive(Copy, Clone, Debug)]
pub enum Operation {
    BoolSet(bool),
    BoolToggle,
    NumSet(f32),
    NumAdd(f32),
    StrSet(Ustr),
}

impl Rule {
    fn build(
        self,
        name: Ustr,
        ctx: &mut Context,
        all_criteria: &[EngineCriterion],
        criteria_index: &UstrMap<(usize, f32, bool)>,
        response_groups_index: &UstrMap<usize>,
    ) -> (EngineRule, Vec<(Ustr, f32)>) {
        // Generate some rudimentary type info
        let mut instructions = UstrMap::default();
        for instruction in &self.instructions {
            let infered_type = match instruction.operation {
                Operation::BoolSet(_) | Operation::BoolToggle => Type::Bool,
                Operation::NumSet(_) | Operation::NumAdd(_) => Type::Num,
                Operation::StrSet(_) => Type::Str,
            };
            let usage = VariableUsage {
                infered_type,
                location: VariableLocation::Rule(name),
            };
            if let Some(variable_usages) = ctx.variable_usages.get_mut(&instruction.variable) {
                variable_usages.push(usage);
            } else {
                ctx.variable_usages
                    .insert(instruction.variable, vec![usage]);
            }
            instructions.insert(
                instruction.variable,
                (instruction.global, instruction.operation),
            );
        }

        // Finalize
        let mut score = 0.0;
        let mut criteria = Vec::new();
        let mut response_groups = Vec::new();
        let mut partition_key = Vec::new();
        let mut used_variables = UstrSet::default();
        let mut repeated_variables = UstrSet::default();

        for criterion_name in self.criteria {
            if let Some((i, weight, partition)) = criteria_index.get(&criterion_name) {
                let criterion = &all_criteria[*i];
                if used_variables.insert(criterion.variable) {
                    score += weight;
                    if *partition {
                        partition_key.push((criterion.variable, criterion.min));
                    } else {
                        criteria.push(*i);
                    }
                } else {
                    // This prevents us from emitting duplicate errors if used more than twice
                    if repeated_variables.insert(criterion.variable) {
                        ctx.errors.push(CompileError::RepeatedVariable {
                            criterion_name,
                            in_rule: name,
                        });
                    }
                }
            } else {
                ctx.errors.push(CompileError::MissingCriterion {
                    criterion_name,
                    in_rule: name,
                });
            }
        }

        for response_group in self.response_groups {
            if let Some(i) = response_groups_index.get(&response_group) {
                response_groups.push(*i);
            } else {
                ctx.errors.push(CompileError::MissingResponseGroup {
                    group_name: response_group,
                    in_rule: name,
                });
            }
        }

        criteria.sort_by_key(|i| all_criteria[*i].variable);
        partition_key.sort_by_key(|(var, _)| *var);

        let engine = EngineRule {
            criteria,
            response_groups,
            instructions,
            score,
            enabled: true,
        };

        (engine, partition_key)
    }
}

#[derive(Debug)]
pub struct ResponseGroup {
    pub delivery: Delivery,
    pub responses: Vec<UstrMap<String>>,
}

#[derive(Debug)]
pub enum Delivery {
    Shuffle, // Random order, uses each response once before repeating
    Random,  // Random order, no restrictions on repetition
    Deplete, // Random order, never repeats a response
    Loop,    // Sequential order, repeats cylically
    List,    // Sequential order, never repeats a response
}

impl ResponseGroup {
    fn build(self, name: Ustr, ctx: &mut Context) -> EngineResponseGroup {
        let weight_ustr = Ustr::from("weight");
        let (weights, responses): (Vec<_>, Vec<_>) = self
            .responses
            .into_iter()
            .map(|mut properties| {
                let weight = properties
                    .remove(&weight_ustr)
                    .and_then(|string| match string.parse::<f32>() {
                        Ok(w) => Some(w),
                        Err(_) => {
                            let error = CompileError::InvalidWeightString {
                                string,
                                in_response_group: name,
                            };
                            ctx.errors.push(error);
                            None
                        }
                    })
                    .unwrap_or(1.0);
                (weight, properties)
            })
            .unzip();
        let dispatcher = match self.delivery {
            Delivery::Shuffle => ResponseDispatcher::Shuffle {
                weights,
                candidates: (0..responses.len()).collect(),
            },
            Delivery::Random => ResponseDispatcher::Random { weights },
            Delivery::Deplete => ResponseDispatcher::Deplete {
                weights,
                candidates: (0..responses.len()).collect(),
            },
            Delivery::Loop => ResponseDispatcher::Loop {
                len: responses.len(),
                index: 0,
            },
            Delivery::List => ResponseDispatcher::List {
                len: responses.len(),
                index: 0,
            },
        };
        EngineResponseGroup {
            dispatcher,
            responses,
        }
    }
}

#[derive(Debug, Default)]
pub struct ResponseEngineCompiler {
    partition_variables: UstrSet,
    criteria: UstrMap<Criterion>,
    rules: UstrMap<Rule>,
    response_groups: UstrMap<ResponseGroup>,
}

#[derive(Default)]
pub struct CompilerReport {
    pub errors: Vec<CompileError>,
}

#[derive(Debug)]
pub enum CompileError {
    IndeterminateVariableType {
        variable_name: Ustr,
        usages: Vec<VariableUsage>,
    },
    InvalidWeightString {
        string: String,
        in_response_group: Ustr,
    },
    MissingCriterion {
        criterion_name: Ustr,
        in_rule: Ustr,
    },
    MissingResponseGroup {
        group_name: Ustr,
        in_rule: Ustr,
    },
    RepeatedVariable {
        criterion_name: Ustr,
        in_rule: Ustr,
    },
}

#[derive(Debug)]
pub enum VariableLocation {
    Criterion(Ustr),
    Rule(Ustr),
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum Type {
    Bool,
    Num,
    Str,
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Type::Bool => write!(f, "boolean"),
            Type::Num => write!(f, "number"),
            Type::Str => write!(f, "string"),
        }
    }
}

#[derive(Debug)]
pub struct VariableUsage {
    pub infered_type: Type,
    pub location: VariableLocation,
}

#[derive(Default)]
struct Context {
    errors: Vec<CompileError>,
    encoder: Encoder,
    // Map from names to types and call-sites
    variable_usages: UstrMap<Vec<VariableUsage>>,
}

impl ResponseEngineCompiler {
    pub fn new() -> ResponseEngineCompiler {
        ResponseEngineCompiler::default()
    }

    pub fn with_partition_variable(&mut self, variable: impl Into<Ustr>) {
        self.partition_variables.insert(variable.into());
    }

    pub fn with_criterion(&mut self, name: impl Into<Ustr>, criterion: Criterion) {
        self.criteria.insert(name.into(), criterion);
    }

    pub fn with_rule(&mut self, name: impl Into<Ustr>, rule: Rule) {
        self.rules.insert(name.into(), rule);
    }

    pub fn with_response_group(&mut self, name: impl Into<Ustr>, response_group: ResponseGroup) {
        self.response_groups.insert(name.into(), response_group);
    }

    pub fn finish(self) -> (Option<ResponseEngine>, CompilerReport) {
        let mut ctx = Context::default();

        // Compile criteria
        let mut criteria = Vec::new();
        let mut criteria_index = UstrMap::default();
        for (i, (name, criterion)) in self.criteria.into_iter().enumerate() {
            let weight = criterion.weight;
            let criterion = criterion.build(name, &mut ctx);
            // If this the criterion is an exact equalitry and the variable is
            // in the partitions list, it can be used to group rules into
            // partitions.
            let partition = criterion.min == criterion.max
                && self.partition_variables.contains(&criterion.variable);
            criteria.push(criterion);
            criteria_index.insert(name, (i, weight, partition));
        }

        // Compile response groups
        let mut response_groups = Vec::new();
        let mut response_group_index = UstrMap::default();
        for (i, (name, response_group)) in self.response_groups.into_iter().enumerate() {
            let response_group = response_group.build(name, &mut ctx);
            response_groups.push(response_group);
            response_group_index.insert(name, i);
        }

        let mut partition_variables: Vec<_> = self.partition_variables.into_iter().collect();
        partition_variables.sort();

        // Compile rules and group into partitions
        let mut rules = RulePartitions {
            vars: partition_variables,
            partitions: HashMap::default(),
        };
        for (name, rule) in self.rules.into_iter() {
            let (rule, assignments) = rule.build(
                name,
                &mut ctx,
                &criteria,
                &criteria_index,
                &response_group_index,
            );
            let key = rules.get_partition_key_for_assignments(&assignments);
            rules.partitions.entry(key).or_default().push(rule);
        }

        // Sort rule partitions by score
        for partition in rules.partitions.values_mut() {
            partition.sort_unstable_by(|ra, rb| rb.score.total_cmp(&ra.score));
        }

        // Rudimentary type-checking
        for (variable_name, usages) in ctx.variable_usages {
            // Check that each variable has a single type
            let coherent = usages
                .windows(2)
                .all(|w| w[0].infered_type == w[1].infered_type);
            if !coherent {
                ctx.errors.push(CompileError::IndeterminateVariableType {
                    variable_name,
                    usages,
                });
            }
        }

        if ctx.errors.is_empty() {
            let engine = ResponseEngine {
                criteria,
                rules,
                response_groups,
                encoder: ctx.encoder,
            };

            (Some(engine), CompilerReport { errors: ctx.errors })
        } else {
            (None, CompilerReport { errors: ctx.errors })
        }
    }
}
