use crate::sqltext::{read_sql_statements, resolve_placeholders, sql_statements};
use crate::tracer::{trace_transaction, TxLockTrace};
use anyhow::anyhow;
use postgres::{Client, NoTls};
use std::collections::HashMap;

pub mod lock_modes;
pub mod locks;
pub mod relkinds;
pub mod sqltext;
pub mod tracer;

pub struct ConnectionSettings {
    user: String,
    database: String,
    host: String,
    port: u16,
    password: String,
}

impl ConnectionSettings {
    pub fn connection_string(&self) -> String {
        format!(
            "host={} user={} dbname={} port={} password={}",
            self.host, self.user, self.database, self.port, self.password
        )
    }
    pub fn new(user: String, database: String, host: String, port: u16, password: String) -> Self {
        ConnectionSettings {
            user,
            database,
            host,
            port,
            password,
        }
    }
}

pub struct TraceSettings<'a> {
    path: String,
    commit: bool,
    placeholders: HashMap<&'a str, &'a str>,
}

impl <'a> TraceSettings<'a> {
    pub fn new(path: String, commit: bool, placeholders: &'a [String]) -> Result<TraceSettings<'a>, anyhow::Error> {
        Ok(TraceSettings {
            path,
            commit,
            placeholders: parse_placeholders(placeholders)?,
        })
    }
}

pub fn parse_placeholders(placeholders: &[String]) -> anyhow::Result<HashMap<&str, &str>> {
    let mut map = HashMap::new();
    for placeholder in placeholders {
        let parts: Vec<&str> = placeholder.splitn(2, '=').collect();
        if parts.len() != 2 {
            return Err(anyhow!("Invalid placeholder: {}", placeholder));
        }
        map.insert(parts[0], parts[1]);
    }
    Ok(map)
}

pub fn perform_trace(
    trace: &TraceSettings,
    connection_settings: &ConnectionSettings,
) -> anyhow::Result<TxLockTrace> {
    let script_content = read_sql_statements(&trace.path)?;
    let sql_script = resolve_placeholders(&script_content, &trace.placeholders)?;
    let sql_statements = sql_statements(&sql_script);
    let mut conn = Client::connect(connection_settings.connection_string().as_str(), NoTls)?;
    let mut tx = conn.transaction()?;
    let trace_result = trace_transaction(&mut tx, sql_statements.iter())?;
    if trace.commit {
        tx.commit()?;
    } else {
        tx.rollback()?;
    }
    Ok(trace_result)
}
