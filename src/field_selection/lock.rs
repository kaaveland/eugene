use serde::Serialize;

use crate::locks::Lock;

#[derive(Serialize, Debug, Eq, PartialEq)]
pub struct TerseLock<'a> {
    mode: &'static str,
    schema: &'a str,
    object_name: &'a str,
}

impl<'a> From<&'a Lock> for TerseLock<'a> {
    fn from(value: &'a Lock) -> Self {
        TerseLock {
            mode: value.mode.to_db_str(),
            schema: value.target().schema.as_str(),
            object_name: value.target().object_name.as_str(),
        }
    }
}

#[derive(Serialize, Debug, Eq, PartialEq)]
pub struct NormalLock<'a> {
    #[serde(flatten)]
    terse: TerseLock<'a>,
    blocked_queries: Vec<&'a str>,
}

impl<'a> From<&'a Lock> for NormalLock<'a> {
    fn from(value: &'a Lock) -> Self {
        NormalLock {
            terse: value.into(),
            blocked_queries: value.blocked_queries(),
        }
    }
}

#[derive(Serialize, Debug, Eq, PartialEq)]
pub struct VerboseLock<'a> {
    #[serde(flatten)]
    normal: NormalLock<'a>,
    rel_kind: &'a str,
    blocked_ddl: Vec<&'a str>,
}

impl<'a> From<&'a Lock> for VerboseLock<'a> {
    fn from(value: &'a Lock) -> Self {
        VerboseLock {
            normal: value.into(),
            rel_kind: value.target().rel_kind.as_str(),
            blocked_ddl: value.mode.blocked_ddl(),
        }
    }
}
