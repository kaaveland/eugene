use crate::lints::ast::AstError;
use crate::pg_types::locks::InvalidLockError;
use handlebars::RenderError;
use serde::de::StdError;
use std::fmt::{Display, Formatter};
use std::num::ParseIntError;
use std::sync::mpsc::{RecvError, SendError};

#[derive(Debug)]
pub struct Error {
    context: Vec<String>,
    pub inner: InnerError,
}

impl<E> From<E> for Error
where
    E: Into<InnerError>,
{
    fn from(e: E) -> Self {
        Error {
            context: vec![],
            inner: e.into(),
        }
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.inner)?;
        for ctx in &self.context {
            write!(f, "\n  {}", ctx)?;
        }
        Ok(())
    }
}

impl StdError for Error {}

pub trait ContextualError {
    fn with_context<S: Into<String>>(self, ctx: S) -> Error;
}
pub trait ContextualResult<T, E> {
    fn with_context<S: Into<String>>(self, ctx: S) -> Result<T, Error>;
}

impl<T> ContextualError for T
where
    T: Into<InnerError>,
{
    fn with_context<S: Into<String>>(self, ctx: S) -> Error {
        Error {
            context: vec![ctx.into()],
            inner: self.into(),
        }
    }
}

impl<T, E> ContextualResult<T, E> for Result<T, E>
where
    E: Into<InnerError>,
{
    fn with_context<S: Into<String>>(self, ctx: S) -> Result<T, Error> {
        self.map_err(|e| e.into().with_context(ctx))
    }
}

impl ContextualError for Error {
    fn with_context<S: Into<String>>(mut self, ctx: S) -> Error {
        self.context.push(ctx.into());
        self
    }
}

impl<T> ContextualResult<T, Error> for Result<T, Error> {
    fn with_context<S: Into<String>>(self, ctx: S) -> Result<T, Error> {
        self.map_err(|e| e.with_context(ctx))
    }
}

#[derive(Debug)]
pub enum InnerError {
    #[allow(dead_code)]
    SqlText(pg_query::Error),
    IO(std::io::Error),
    PathParseError,
    NotFound,
    NotValidUtf8,
    DifferentScriptNameTypes,
    NotSortableScriptNames,
    UnknownPathType,
    InvalidSortMode,
    UnresolvedPlaceHolder,
    UnableToInitDb,
    AstInterpretationError(AstError),
    Template(RenderError),
    BadCommentInstruction(String),
    ScriptParsingError(String),
    InvalidContype(char),
    InvalidLock(InvalidLockError),
    PostgresError(postgres::Error),
    MissingRequiredCommand(String),
    PlaceholderSyntaxError,
    PgPassSyntaxError,
    PgPassFileNotFound,
    InvalidNumber(ParseIntError),
    InvalidUnit(String),
    MissingCaptureError,
    MissingStdout,
    RecvError(RecvError),
    SendError,
    SerdeError(serde_json::Error),
    PgPassEntryNotFound,
    InvalidGitMode,
    NoGitExecutableError,
    NoGitRepositoryError,
    GitExecutionError,
    GitError,
    InvalidPath,
}

impl From<serde_json::Error> for InnerError {
    fn from(value: serde_json::Error) -> Self {
        InnerError::SerdeError(value)
    }
}

impl From<RecvError> for InnerError {
    fn from(value: RecvError) -> Self {
        InnerError::RecvError(value)
    }
}

impl<T> From<SendError<T>> for InnerError {
    fn from(_value: SendError<T>) -> Self {
        InnerError::SendError
    }
}

impl From<ParseIntError> for InnerError {
    fn from(value: ParseIntError) -> Self {
        InnerError::InvalidNumber(value)
    }
}
impl From<postgres::Error> for InnerError {
    fn from(value: postgres::Error) -> Self {
        InnerError::PostgresError(value)
    }
}

impl From<InvalidLockError> for InnerError {
    fn from(value: InvalidLockError) -> Self {
        InnerError::InvalidLock(value)
    }
}

impl From<RenderError> for InnerError {
    fn from(e: RenderError) -> Self {
        InnerError::Template(e)
    }
}

impl From<pg_query::Error> for InnerError {
    fn from(e: pg_query::Error) -> Self {
        InnerError::SqlText(e)
    }
}

impl From<std::io::Error> for InnerError {
    fn from(e: std::io::Error) -> Self {
        InnerError::IO(e)
    }
}

impl From<AstError> for InnerError {
    fn from(value: AstError) -> Self {
        InnerError::AstInterpretationError(value)
    }
}
