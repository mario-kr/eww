use crate::{
    expr::{Expr, ExprType, Span},
    lexer,
};
use codespan_reporting::{diagnostic, files};
use thiserror::Error;

pub type AstResult<T> = Result<T, AstError>;

#[derive(Debug, PartialEq, Eq, Error)]
pub enum AstError {
    #[error("Definition invalid")]
    InvalidDefinition(Option<Span>),
    #[error("Expected a {1}, but got nothing")]
    MissingNode(Option<Span>, ExprType),
    #[error("Wrong type of expression: Expected {1} but got {2}")]
    WrongExprType(Option<Span>, ExprType, ExprType),

    #[error("Parse error: {source}")]
    ParseError { file_id: Option<usize>, source: lalrpop_util::ParseError<usize, lexer::Token, lexer::LexicalError> },
}

impl AstError {
    pub fn get_span(&self) -> Option<Span> {
        match self {
            AstError::InvalidDefinition(span) => *span,
            AstError::MissingNode(span, _) => *span,
            AstError::WrongExprType(span, ..) => *span,
            AstError::ParseError { file_id, source } => file_id.and_then(|id| get_parse_error_span(id, source)),
        }
    }

    pub fn pretty_diagnostic(&self, files: &files::SimpleFiles<&str, &str>) -> diagnostic::Diagnostic<usize> {
        let diag = diagnostic::Diagnostic::error().with_message(format!("{}", self));
        if let Some(span) = self.get_span() {
            diag.with_labels(vec![diagnostic::Label::primary(span.2, span.0..span.1)])
        } else {
            diag
        }
    }

    pub fn from_parse_error(file_id: usize, err: lalrpop_util::ParseError<usize, lexer::Token, lexer::LexicalError>) -> AstError {
        AstError::ParseError { file_id: Some(file_id), source: err }
    }
}

fn get_parse_error_span(
    file_id: usize,
    err: &lalrpop_util::ParseError<usize, lexer::Token, lexer::LexicalError>,
) -> Option<Span> {
    match err {
        lalrpop_util::ParseError::InvalidToken { location } => Some(Span(*location, *location, file_id)),
        lalrpop_util::ParseError::UnrecognizedEOF { location, expected } => Some(Span(*location, *location, file_id)),
        lalrpop_util::ParseError::UnrecognizedToken { token, expected } => Some(Span(token.0, token.2, file_id)),
        lalrpop_util::ParseError::ExtraToken { token } => Some(Span(token.0, token.2, file_id)),
        lalrpop_util::ParseError::User { error } => None,
    }
}

pub fn spanned(span: Span, err: impl Into<AstError>) -> AstError {
    use AstError::*;
    match err.into() {
        AstError::InvalidDefinition(None) => AstError::InvalidDefinition(Some(span)),
        AstError::MissingNode(None, x) => AstError::MissingNode(Some(span), x),
        AstError::WrongExprType(None, x, y) => AstError::WrongExprType(Some(span), x, y),
        x => x,
    }
}

pub trait OptionAstErrorExt<T> {
    fn or_missing(self, t: ExprType) -> Result<T, AstError>;
}
impl<T> OptionAstErrorExt<T> for Option<T> {
    fn or_missing(self, t: ExprType) -> Result<T, AstError> {
        self.ok_or(AstError::MissingNode(None, t))
    }
}

pub trait AstResultExt<T> {
    fn at(self, span: Span) -> Result<T, AstError>;
}

impl<T, E: Into<AstError>> AstResultExt<T> for Result<T, E> {
    fn at(self, span: Span) -> Result<T, AstError> {
        self.map_err(|err| spanned(span, err))
    }
}

#[macro_export]
macro_rules! spanned {
    ($span:expr, $block:expr) => {{
        let span = $span;
        let result: Result<_, AstError> = try { $block };
        result.at(span)
    }};
}
