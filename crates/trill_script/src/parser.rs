use logos::Lexer;
use logos::Span;
use ustr::Ustr;

use trill_core::Criterion;
use trill_core::Delivery;
use trill_core::Instruction;
use trill_core::Operation;
use trill_core::Predicate;
use trill_core::ResponseGroup;
use trill_core::Rule;
use ustr::UstrMap;

use crate::error::AddSpan;
use crate::error::ParseError;
use crate::error::Spanned;
use crate::lexer::Token;

#[derive(Debug)]
pub enum Definition {
    Criterion {
        name: Ustr,
        criterion: Criterion,
    },
    Rule {
        name: Ustr,
        rule: Rule,
    },
    ResponseGroup {
        name: Ustr,
        response_group: ResponseGroup,
    },
}

impl Token {
    fn expect_number(self) -> Result<f32, ParseError> {
        if let Token::Number(number) = self {
            Ok(number)
        } else {
            Err(ParseError::UnexpectedToken {
                token: self,
                expected: "a number literal",
                hint: None,
            })
        }
    }

    fn expect_string(self) -> Result<String, ParseError> {
        if let Token::String(string) = self {
            Ok(string)
        } else {
            Err(ParseError::UnexpectedToken {
                token: self,
                expected: "a string literal",
                hint: Some("string literals must be enclosed in quotes"),
            })
        }
    }

    fn expect_symbol(self) -> Result<Ustr, ParseError> {
        if let Token::Symbol(ustr) = self {
            Ok(ustr)
        } else {
            Err(ParseError::UnexpectedToken {
                token: self,
                expected: "a symbol",
                hint: None,
            })
        }
    }

    fn expect_paren_open(self) -> Result<(), ParseError> {
        if Token::ParenOpen == self {
            Ok(())
        } else {
            Err(ParseError::UnexpectedToken {
                token: self,
                expected: "an open parenthesis",
                hint: None,
            })
        }
    }

    fn expect_paren_close(self) -> Result<(), ParseError> {
        if Token::ParenClose == self {
            Ok(())
        } else {
            Err(ParseError::UnexpectedToken {
                token: self,
                expected: "a closing parenthesis",
                hint: None,
            })
        }
    }
}

trait ExpectUstrExt {
    fn expect_ident(self) -> Result<Ustr, ParseError>;
    fn expect_var(self) -> Result<Ustr, ParseError>;
}

impl ExpectUstrExt for Ustr {
    fn expect_ident(self) -> Result<Ustr, ParseError> {
        let first_char = self.chars().next().unwrap();
        if first_char.is_ascii_uppercase() {
            Ok(self)
        } else {
            Err(ParseError::UnexpectedToken {
                token: Token::Symbol(self),
                expected: "an identifier",
                hint: Some("identifiers must begin with an upper-case ascii letter"),
            })
        }
    }

    fn expect_var(self) -> Result<Ustr, ParseError> {
        let first_char = self.chars().next().unwrap();
        if first_char.is_ascii_lowercase() {
            Ok(self)
        } else {
            Err(ParseError::UnexpectedToken {
                token: Token::Symbol(self),
                expected: "a variable name",
                hint: Some("variable names must begin with a lower-case ascii letter"),
            })
        }
    }
}

pub struct Parser<'src> {
    lexer: Lexer<'src, Token>,
}

impl<'src> Parser<'src> {
    pub fn new(src: &'src str) -> Parser<'src> {
        Parser {
            lexer: Lexer::new(src),
        }
    }

    pub fn maybe_parse_definition(
        &mut self,
    ) -> Result<Option<(Definition, Span)>, Spanned<ParseError>> {
        match self.lexer.next() {
            Some(Ok(Token::ParenOpen)) => {
                let start = self.lexer.span().start;
                let def = self.parse_definition()?;
                let end = self.lexer.span().end;
                Ok(Some((def, start..end)))
            }
            Some(Ok(token)) => Err(Spanned {
                error: ParseError::UnexpectedToken {
                    token,
                    expected: "either an open parenthesis or the end of the file",
                    hint: None,
                },
                span: self.lexer.span(),
            }),
            Some(Err(Spanned { span, error })) => Err(Spanned {
                error: ParseError::LexError { error },
                span,
            }),
            None => Ok(None),
        }
    }

    fn parse_definition(&mut self) -> Result<Definition, Spanned<ParseError>> {
        let symbol = self
            .parse_token()?
            .expect_symbol()
            .span(self.lexer.span())?;

        let name = self
            .parse_token()?
            .expect_symbol()
            .and_then(|s| s.expect_ident())
            .span(self.lexer.span())?;

        match symbol.as_str() {
            "criterion" => {
                let criterion = self.parse_criterion()?;
                Ok(Definition::Criterion { name, criterion })
            }
            "rule" => {
                let rule = self.parse_rule()?;
                Ok(Definition::Rule { name, rule })
            }
            "response" => {
                let response_group = self.parse_response_group()?;
                Ok(Definition::ResponseGroup {
                    name,
                    response_group,
                })
            }
            _ => Err(Spanned {
                error: ParseError::UnexpectedToken {
                    token: Token::Symbol(symbol),
                    expected: "a symbol containing one of the keywords 'criterion', 'rule', or 'response'",
                    hint: None,
                },
                span: self.lexer.span(),
            }),
        }
    }

    fn parse_criterion(&mut self) -> Result<Criterion, Spanned<ParseError>> {
        self.parse_token()?
            .expect_paren_open()
            .span(self.lexer.span())?;
        let variable = self
            .parse_token()?
            .expect_symbol()
            .and_then(|s| s.expect_var())
            .span(self.lexer.span())?;
        let predicate = self.parse_predicate()?;

        // This is written as a loop to allow for additional keywords to be added here
        let mut weight = None;
        loop {
            let token = self.parse_token()?;
            match token {
                Token::ParenClose => break,
                Token::Symbol(s) if s == "weight" && weight.is_none() => {
                    weight = Some(
                        self.parse_token()?
                            .expect_number()
                            .span(self.lexer.span())?,
                    );
                }
                _ => {
                    return Err(Spanned {
                        error: ParseError::UnexpectedToken {
                            token,
                            expected: "either a closing parenthesis, or a symbol containing the either of the keywords 'optional' or 'weight'",
                            hint: None,
                        },
                        span: self.lexer.span(),
                    });
                }
            }
        }

        let criterion = Criterion {
            variable,
            predicate,
            weight: weight.unwrap_or(1.0),
        };

        Ok(criterion)
    }

    fn parse_predicate(&mut self) -> Result<Predicate, Spanned<ParseError>> {
        match self.parse_token()? {
            Token::DoubleEqual => match self.parse_token()? {
                Token::Symbol(s) if s == "true" => {
                    self.parse_token()?
                        .expect_paren_close()
                        .span(self.lexer.span())?;
                    Ok(Predicate::BoolEqual(true))
                }
                Token::Symbol(s) if s == "false" => {
                    self.parse_token()?
                        .expect_paren_close()
                        .span(self.lexer.span())?;
                    Ok(Predicate::BoolEqual(false))
                }
                Token::Symbol(symbol) => {
                    self.parse_token()?
                        .expect_paren_close()
                        .span(self.lexer.span())?;
                    Ok(Predicate::StrEqual(symbol))
                }
                Token::Number(value) => {
                    self.parse_token()?
                        .expect_paren_close()
                        .span(self.lexer.span())?;
                    Ok(Predicate::NumEqual(value))
                }
                token => Err(Spanned {
                    error: ParseError::UnexpectedToken {
                        token,
                        expected: "eeither a boolean literal, a numeric literal, or a symbol",
                        hint: None,
                    },
                    span: self.lexer.span(),
                }),
            },
            Token::Symbol(s) if s == "in" => match self.parse_token()? {
                Token::Number(start) => {
                    let inclusive = match self.parse_token()? {
                        Token::Range(inclusive) => inclusive,
                        token => {
                            return Err(Spanned {
                                error: ParseError::UnexpectedToken {
                                    token,
                                    expected: "either of the specifiers '..' or '..='",
                                    hint: None,
                                },
                                span: self.lexer.span(),
                            });
                        }
                    };
                    match self.parse_token()? {
                        Token::Number(mut end) => {
                            self.parse_token()?
                                .expect_paren_close()
                                .span(self.lexer.span())?;
                            if !inclusive {
                                end = end.next_down();
                            }
                            Ok(Predicate::NumRange(Some(start), Some(end)))
                        }
                        Token::ParenClose => Ok(Predicate::NumRange(Some(start), None)),
                        token => Err(Spanned {
                            error: ParseError::UnexpectedToken {
                                token,
                                expected: "either a numeric literal or a closing parenthesis",
                                hint: None,
                            },
                            span: self.lexer.span(),
                        }),
                    }
                }
                Token::Range(true) => {
                    let end = self
                        .parse_token()?
                        .expect_number()
                        .span(self.lexer.span())?;
                    self.parse_token()?
                        .expect_paren_close()
                        .span(self.lexer.span())?;
                    Ok(Predicate::NumRange(None, Some(end)))
                }
                Token::Range(false) => match self.parse_token()? {
                    Token::Number(end) => {
                        self.parse_token()?
                            .expect_paren_close()
                            .span(self.lexer.span())?;
                        let end = end.next_down();
                        Ok(Predicate::NumRange(None, Some(end)))
                    }
                    Token::ParenClose => Ok(Predicate::NumRange(None, None)),
                    token => Err(Spanned {
                        error: ParseError::UnexpectedToken {
                            token,
                            expected: "either a numeric literal or a closing parenthesis",
                            hint: None,
                        },
                        span: self.lexer.span(),
                    }),
                },
                token => Err(Spanned {
                    error: ParseError::UnexpectedToken {
                        token,
                        expected: "either a numeric literal or either of the specifiers '..' or '..='",
                        hint: None,
                    },
                    span: self.lexer.span(),
                }),
            },
            token => Err(Spanned {
                error: ParseError::UnexpectedToken {
                    token,
                    expected: "either a symbol containing the keyword 'in' or the specifier '=='",
                    hint: None,
                },
                span: self.lexer.span(),
            }),
        }
    }

    fn parse_list<T>(
        &mut self,
        parse_item: fn(token: Token) -> Result<T, ParseError>,
    ) -> Result<Vec<T>, Spanned<ParseError>> {
        self.parse_token()?
            .expect_paren_open()
            .span(self.lexer.span())?;
        let mut list = Vec::new();
        loop {
            let token = self.parse_token()?;
            if token == Token::ParenClose {
                return Ok(list);
            } else {
                let item = parse_item(token).span(self.lexer.span())?;
                list.push(item)
            }
        }
    }

    fn parse_ident_list(&mut self) -> Result<Vec<Ustr>, Spanned<ParseError>> {
        self.parse_list(|token| token.expect_symbol()?.expect_ident())
    }

    fn parse_operation(&mut self) -> Result<Operation, Spanned<ParseError>> {
        match self.parse_token()? {
            Token::ColonNegated => Ok(Operation::BoolToggle),
            Token::ColonEqual => match self.parse_token()? {
                Token::Symbol(symbol) if symbol == "true" => Ok(Operation::BoolSet(true)),
                Token::Symbol(symbol) if symbol == "false" => Ok(Operation::BoolSet(false)),
                Token::Number(value) => Ok(Operation::NumSet(value)),
                Token::Symbol(symbol) => Ok(Operation::StrSet(symbol)),
                token => Err(Spanned {
                    error: ParseError::UnexpectedToken {
                        token,
                        expected: "either a boolean literal, a numeric literal, or a symbol",
                        hint: None,
                    },
                    span: self.lexer.span(),
                }),
            },
            Token::ColonPlus => {
                let value = self
                    .parse_token()?
                    .expect_number()
                    .span(self.lexer.span())?;
                Ok(Operation::NumAdd(value))
            }
            Token::ColonMinus => {
                let value = self
                    .parse_token()?
                    .expect_number()
                    .span(self.lexer.span())?;
                Ok(Operation::NumAdd(-value))
            }
            token => Err(Spanned {
                error: ParseError::UnexpectedToken {
                    token,
                    expected: "one of the operators ':!', ':=', ':+' or ':-'",
                    hint: None,
                },
                span: self.lexer.span(),
            }),
        }
    }

    fn parse_rule(&mut self) -> Result<Rule, Spanned<ParseError>> {
        let criteria = self.parse_ident_list()?;
        let response_groups = self.parse_ident_list()?;

        let mut instructions = Vec::new();
        loop {
            match self.parse_token()? {
                Token::ParenClose => break,
                Token::DollarSign => {
                    let variable = self
                        .parse_token()?
                        .expect_symbol()
                        .and_then(|s| s.expect_var())
                        .span(self.lexer.span())?;
                    let operation = self.parse_operation()?;
                    instructions.push(Instruction {
                        variable,
                        global: true,
                        operation,
                    });
                }
                Token::Symbol(var) => {
                    let variable = var.expect_var().span(self.lexer.span())?;
                    let operation = self.parse_operation()?;
                    instructions.push(Instruction {
                        variable,
                        global: false,
                        operation,
                    });
                }
                token => {
                    return Err(Spanned {
                        error: ParseError::UnexpectedToken {
                            token,
                            expected: "either a variable name, the '$' variable modifier, or a closing parenthesis",
                            hint: None,
                        },
                        span: self.lexer.span(),
                    });
                }
            }
        }

        let rule = Rule {
            criteria,
            instructions,
            response_groups,
        };

        Ok(rule)
    }

    fn parse_response(&mut self) -> Result<UstrMap<String>, Spanned<ParseError>> {
        let mut response = UstrMap::default();
        loop {
            match self.parse_token()? {
                Token::ParenClose => break,
                Token::Symbol(key) => {
                    let value = self
                        .parse_token()?
                        .expect_string()
                        .span(self.lexer.span())?;
                    response.insert(key, value);
                }
                token => {
                    return Err(Spanned {
                        error: ParseError::UnexpectedToken {
                            token,
                            expected: "either a symbol or a closing parenthesis",
                            hint: None,
                        },
                        span: self.lexer.span(),
                    });
                }
            }
        }
        Ok(response)
    }

    fn parse_response_group(&mut self) -> Result<ResponseGroup, Spanned<ParseError>> {
        let mut token = self.parse_token()?;

        let delivery = if let Token::Symbol(symbol) = token {
            token = self.parse_token()?;
            match symbol.as_str() {
                "shuffle" => Delivery::Shuffle,
                "random" => Delivery::Random,
                "deplete" => Delivery::Deplete,
                "loop" => Delivery::Loop,
                "list" => Delivery::List,
                _ => {
                    return Err(Spanned {
                        error: ParseError::UnexpectedToken {
                            token: Token::Symbol(symbol),
                            expected: "a symbol containing one of the keywords 'shuffle', 'random', 'deplete', 'loop', or 'list'",
                            hint: None,
                        },
                        span: self.lexer.span(),
                    });
                }
            }
        } else {
            Delivery::Shuffle
        };

        let mut responses = Vec::new();
        loop {
            match token {
                Token::ParenClose if !responses.is_empty() => break,
                Token::ParenOpen => {
                    let response = self.parse_response()?;
                    responses.push(response);
                }
                token => {
                    return Err(Spanned {
                        error: ParseError::UnexpectedToken {
                            token,
                            expected: "either open parenthesis or a closing parenthesis",
                            hint: None,
                        },
                        span: self.lexer.span(),
                    });
                }
            }
            token = self.parse_token()?;
        }

        let response_group = ResponseGroup {
            delivery,
            responses,
        };

        Ok(response_group)
    }

    fn parse_token(&mut self) -> Result<Token, Spanned<ParseError>> {
        match self.lexer.next() {
            Some(Ok(token)) => Ok(token),
            Some(Err(Spanned { span, error })) => Err(Spanned {
                error: ParseError::LexError { error },
                span,
            }),
            None => Err(Spanned {
                error: ParseError::UnexpectedEof,
                span: self.lexer.span(),
            }),
        }
    }
}
