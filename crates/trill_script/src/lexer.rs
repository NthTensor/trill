use std::fmt;

use logos::Lexer;
use logos::Logos;
use ustr::Ustr;

use crate::error::LexicalError;
use crate::error::Spanned;

#[derive(Logos, Debug, PartialEq, Clone)]
#[logos(skip r"[ \t\n\f]+")]
#[logos(error(Spanned<LexicalError>, callback = parse_error))]
pub enum Token {
    #[regex("[a-zA-Z][a-zA-Z0-9_$]*", |lex| Ustr::from(lex.slice()))]
    Symbol(Ustr),

    #[regex(r"-?(?:0|[1-9]\d*)(?:\.\d+)?(?:[eE][+-]?\d+)?", parse_numeric)]
    Number(f32),

    #[regex(r#""(?:[^"]|\\")*""#, parse_string)]
    String(String),

    #[token("(")]
    ParenOpen,

    #[token(")")]
    ParenClose,

    #[token(":=")]
    ColonEqual,

    #[token(":!")]
    ColonNegated,

    #[token(":+")]
    ColonPlus,

    #[token(":-")]
    ColonMinus,

    #[token("==")]
    DoubleEqual,

    #[token("..", |_| false)]
    #[token("..=", |_| true)]
    Range(bool),

    #[token("$")]
    DollarSign,
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Token::Symbol(ustr) => write!(f, "symbol '{}'", ustr),
            Token::Number(num) => write!(f, "number '{}'", num),
            Token::String(string) => write!(f, "string literal \"{}\"", string),
            Token::ParenOpen => write!(f, "an open parnehtisis"),
            Token::ParenClose => write!(f, "a closing parenthisis"),
            Token::ColonEqual => write!(f, "the := operator"),
            Token::ColonNegated => write!(f, "the :! operator"),
            Token::ColonPlus => write!(f, "the :+ operator"),
            Token::ColonMinus => write!(f, "the :- operator"),
            Token::DoubleEqual => write!(f, "the == specifier"),
            Token::Range(false) => write!(f, "the .. specifier"),
            Token::Range(true) => write!(f, "the ..= specifier"),
            Token::DollarSign => write!(f, "the $ variable modifier"),
        }
    }
}

fn parse_numeric(lexer: &mut Lexer<Token>) -> Result<f32, Spanned<LexicalError>> {
    lexer.slice().parse::<f32>().map_err(|error| Spanned {
        error: LexicalError::NumericError { error },
        span: lexer.span(),
    })
}

fn parse_string(lexer: &mut Lexer<Token>) -> Result<String, Spanned<LexicalError>> {
    let str = lexer.slice();
    let inner_content = &str[1..str.len() - 1];
    Ok(inner_content.to_string())
}

fn parse_error(lexer: &mut Lexer<Token>) -> Spanned<LexicalError> {
    Spanned {
        error: LexicalError::LexicalError,
        span: lexer.span(),
    }
}
