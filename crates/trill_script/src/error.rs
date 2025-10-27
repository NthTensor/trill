use std::num::ParseFloatError;
use std::ops::Range;

use codespan_reporting::{
    diagnostic::{Diagnostic, Label},
    files::SimpleFiles,
    term::{
        self,
        termcolor::{ColorChoice, StandardStream},
    },
};
use logos::Span;
use trill_core::{CompileError, VariableLocation};
use ustr::{Ustr, UstrMap};

use crate::lexer::Token;

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Spanned<E> {
    pub error: E,
    pub span: Span,
}

impl<E: Default> Default for Spanned<E> {
    fn default() -> Self {
        Spanned {
            error: E::default(),
            span: Range { start: 0, end: 0 },
        }
    }
}

pub trait AddSpan {
    type Result;

    fn span(self, span: Span) -> Self::Result;
}

impl<T, E> AddSpan for Result<T, E> {
    type Result = Result<T, Spanned<E>>;

    fn span(self, span: Span) -> Self::Result {
        self.map_err(|error| Spanned { error, span })
    }
}

#[derive(Default, Debug, PartialEq, Eq, Clone)]
pub enum LexicalError {
    NumericError {
        error: ParseFloatError,
    },
    #[default]
    LexicalError,
}

#[derive(Debug, PartialEq, Clone)]
pub enum ParseError {
    UnexpectedEof,
    UnexpectedToken {
        token: Token,
        expected: &'static str,
        hint: Option<&'static str>,
    },
    LexError {
        error: LexicalError,
    },
}

impl AddSpan for ParseError {
    type Result = Spanned<ParseError>;

    fn span(self, span: Span) -> Self::Result {
        Spanned { error: self, span }
    }
}

#[derive(Debug)]
pub struct Location {
    pub file_id: usize,
    pub span: Range<usize>,
}

#[derive(Debug)]
pub struct ScriptReport {
    pub compile_errors: Vec<CompileError>,
    pub parse_errors: Vec<(usize, Spanned<ParseError>)>,
    pub files: SimpleFiles<Ustr, String>,
    pub criterion_locations: UstrMap<Location>,
    pub rule_locations: UstrMap<Location>,
    pub response_group_locations: UstrMap<Location>,
}

impl ScriptReport {
    pub fn print(self) {
        let writer = StandardStream::stderr(ColorChoice::Always);
        let config = codespan_reporting::term::Config::default();

        for (file_id, Spanned { error, span }) in self.parse_errors {
            let diagnostic = match error {
                ParseError::UnexpectedEof => Diagnostic::error()
                    .with_message("encountered unexpected end of file while parsing")
                    .with_label(
                        Label::primary(file_id, span).with_message("file ends abruptly here"),
                    ),
                ParseError::UnexpectedToken {
                    token,
                    expected,
                    hint,
                } => {
                    let diagnostic = Diagnostic::error()
                        .with_message("encountered unexpected token while parsing")
                        .with_label(
                            Label::primary(file_id, span)
                                .with_message(format!("expected {}, found {}", expected, token)),
                        );

                    if let Some(hint) = hint {
                        diagnostic.with_note(hint)
                    } else {
                        diagnostic
                    }
                }
                ParseError::LexError { error } => match error {
                    LexicalError::NumericError { error } => Diagnostic::error()
                        .with_message("failed to prase float literal")
                        .with_label(
                            Label::primary(file_id, span).with_message(format!("{}", error)),
                        ),
                    LexicalError::LexicalError => Diagnostic::error()
                        .with_message(format!("lexical error in file {}", file_id)),
                },
            };

            term::emit_to_write_style(&mut writer.lock(), &config, &self.files, &diagnostic)
                .unwrap();
        }

        for compile_error in self.compile_errors {
            let diagnostic = match compile_error {
                CompileError::IndeterminateVariableType {
                    variable_name,
                    usages,
                } => {
                    let labels = usages.into_iter().map(|useage| {
                        let location = match useage.location {
                            VariableLocation::Criterion(ustr) => {
                                self.criterion_locations.get(&ustr).unwrap()
                            }
                            VariableLocation::Rule(ustr) => self.rule_locations.get(&ustr).unwrap(),
                        };
                        Label::secondary(location.file_id, location.span.clone())
                            .with_message(format!("used as {} here", useage.infered_type))
                    });
                    Diagnostic::error()
                        .with_message(format!(
                            "found conflicting types for variable {}",
                            variable_name
                        ))
                        .with_labels_iter(labels)
                }
                CompileError::InvalidWeightString {
                    string,
                    in_response_group,
                } => {
                    let location = self
                        .response_group_locations
                        .get(&in_response_group)
                        .unwrap();
                    Diagnostic::error()
                        .with_message("invalid weight string")
                        .with_label(
                            Label::primary(location.file_id, location.span.clone()).with_message(
                                format!("unable to understand string \"{}\"", string),
                            ),
                        )
                }
                CompileError::MissingCriterion {
                    criterion_name,
                    in_rule,
                } => {
                    let location = self.rule_locations.get(&in_rule).unwrap();
                    Diagnostic::error()
                        .with_message(format!(
                            "unable to fine criteria defintion {}",
                            criterion_name
                        ))
                        .with_label(
                            Label::primary(location.file_id, location.span.clone())
                                .with_message(format!("referenced in rule {}", in_rule)),
                        )
                }
                CompileError::MissingResponseGroup {
                    group_name,
                    in_rule,
                } => {
                    let location = self.rule_locations.get(&in_rule).unwrap();
                    Diagnostic::error()
                        .with_message(format!(
                            "unable to fine response group defintion {}",
                            group_name
                        ))
                        .with_label(
                            Label::primary(location.file_id, location.span.clone())
                                .with_message(format!("referenced in rule {}", in_rule)),
                        )
                }
                CompileError::RepeatedVariable {
                    criterion_name,
                    in_rule,
                } => {
                    let location = self.rule_locations.get(&in_rule).unwrap();
                    Diagnostic::error()
                        .with_message(format!("variable used twice within the same rule",))
                        .with_label(
                            Label::primary(location.file_id, location.span.clone()).with_message(
                                format!(
                                    "criterion {} referenced in rule {}",
                                    criterion_name, in_rule
                                ),
                            ),
                        )
                }
            };

            term::emit_to_write_style(&mut writer.lock(), &config, &self.files, &diagnostic)
                .unwrap();
        }
    }
}
