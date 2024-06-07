use crate::error::{ContextualError, InnerError};
use std::env;

#[cfg(windows)]
fn default_pgpass_path() -> Result<String> {
    if let Ok(path) = env::var("APPDATA") {
        Ok(format!("{}/postgresql/pgpass.conf", path))
    } else {
        Err(InnerError::PgPassFileNotFound.into())
    }
}

#[cfg(not(windows))]
fn default_pgpass_path() -> crate::Result<String> {
    if let Ok(path) = env::var("HOME") {
        Ok(format!("{}/.pgpass", path))
    } else {
        Err(InnerError::PgPassFileNotFound.into())
    }
}

fn pgpass_path() -> crate::Result<String> {
    if let Ok(path) = env::var("PGPASSFILE") {
        Ok(path)
    } else {
        default_pgpass_path()
    }
}

fn read_pgpass() -> crate::Result<String> {
    let path = pgpass_path()?;
    // TODO: Warn and discard if file is not chmodded to 600 on unix
    let contents = std::fs::read_to_string(path)?;
    Ok(contents)
}

#[derive(Debug, Eq, PartialEq, Clone)]
enum PgPassRule<T: Eq + PartialEq + Clone> {
    Match(T),
    Anything,
}

impl PgPassRule<String> {
    fn matches(&self, value: &str) -> bool {
        match self {
            PgPassRule::Match(pattern) => pattern == value,
            PgPassRule::Anything => true,
        }
    }
}

impl PgPassRule<u16> {
    fn matches(&self, value: u16) -> bool {
        match self {
            PgPassRule::Match(pattern) => *pattern == value,
            PgPassRule::Anything => true,
        }
    }
}

#[derive(Debug, Eq, PartialEq, Clone)]
struct PgPassEntry {
    host: PgPassRule<String>,
    port: PgPassRule<u16>,
    database: PgPassRule<String>,
    user: PgPassRule<String>,
    password: String,
}

impl PgPassEntry {
    fn apply_to(&self, host: &str, port: u16, database: &str, user: &str) -> Option<&str> {
        if self.host.matches(host)
            && self.port.matches(port)
            && self.database.matches(database)
            && self.user.matches(user)
        {
            Some(self.password.as_str())
        } else {
            None
        }
    }
}

fn parse_pgpass_entry(line: &str) -> crate::Result<PgPassEntry> {
    let parts: Vec<&str> = line.split(':').collect();
    if parts.len() != 5 {
        return Err(
            InnerError::PgPassSyntaxError.with_context(format!("Invalid pgpass entry: {}", line))
        );
    }
    let host = match parts[0] {
        "*" => PgPassRule::Anything,
        host => PgPassRule::Match(host.to_string()),
    };
    let port = match parts[1] {
        "*" => PgPassRule::Anything,
        port => PgPassRule::Match(port.parse::<u16>()?),
    };
    let database = match parts[2] {
        "*" => PgPassRule::Anything,
        database => PgPassRule::Match(database.to_string()),
    };
    let user = match parts[3] {
        "*" => PgPassRule::Anything,
        user => PgPassRule::Match(user.to_string()),
    };
    Ok(PgPassEntry {
        host,
        port,
        database,
        user,
        password: parts[4].to_string(),
    })
}

fn parse_pgpass_entries(contents: &str) -> crate::Result<PgPassFile> {
    let mut entries = Vec::new();
    for line in contents.lines() {
        if !line.starts_with('#') {
            entries.push(parse_pgpass_entry(line)?);
        }
    }
    Ok(PgPassFile { entries })
}

/// Represents the contents of a pgpass file, see <https://www.postgresql.org/docs/current/libpq-pgpass.html>
#[derive(Debug, Eq, PartialEq, Clone)]
pub struct PgPassFile {
    entries: Vec<PgPassEntry>,
}

/// Reads the pgpass file, see https://www.postgresql.org/docs/current/libpq-pgpass.html
///
/// Will respect the `PGPASSFILE` environment variable if set, otherwise will use the default location
pub fn read_pgpass_file() -> crate::Result<PgPassFile> {
    let contents = read_pgpass()?;
    parse_pgpass_entries(&contents)
}

impl PgPassFile {
    /// Find the password for a given host, port, database and user
    ///
    /// Will always return the password for the first matching pgpass line, if there are overlapping
    /// rules, only the first password will be returned
    pub fn find_password(
        &self,
        host: &str,
        port: u16,
        database: &str,
        user: &str,
    ) -> crate::Result<&str> {
        self.entries
            .iter()
            .find_map(|entry| entry.apply_to(host, port, database, user))
            .ok_or_else(|| {
                InnerError::PgPassEntryNotFound.with_context("No matching pgpass entry found")
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_parse_pgpass_bad_entry() {
        let line = "localhost:5432:mydb:myuser";
        assert!(parse_pgpass_entry(line).is_err());
    }

    #[test]
    fn test_parse_pgpass_wildcards_entry() {
        let line = "*:*:*:myuser:mypass";
        let entry = parse_pgpass_entry(line).unwrap();
        assert_eq!(entry.host, PgPassRule::Anything);
        assert_eq!(entry.port, PgPassRule::Anything);
        assert_eq!(entry.database, PgPassRule::Anything);
        assert_eq!(entry.user, PgPassRule::Match("myuser".to_string()));
        assert_eq!(entry.password, "mypass");
    }

    #[test]
    fn test_parse_unix_socket_host_pgpass_entry() {
        let line = "/var/run/postgresql:*:*:myuser:mypass";
        let entry = parse_pgpass_entry(line).unwrap();
        assert_eq!(
            entry.host,
            PgPassRule::Match("/var/run/postgresql".to_string())
        );
        assert_eq!(entry.port, PgPassRule::Anything);
        assert_eq!(entry.database, PgPassRule::Anything);
        assert_eq!(entry.user, PgPassRule::Match("myuser".to_string()));
        assert_eq!(entry.password, "mypass");
    }

    #[test]
    fn test_pick_correct_password_for_pgpassfile() {
        let contents = r#"localhost:5432:mydb:myuser:mypass
/var/run/postgresql:*:*:myuser:unixsocketpass"#;
        let pgpass = parse_pgpass_entries(contents).unwrap();
        assert!(pgpass
            .find_password("example.com", 5432, "mydb", "myuser")
            .is_err());
        assert_eq!(
            pgpass
                .find_password("/var/run/postgresql", 5432, "mydb", "myuser")
                .unwrap(),
            "unixsocketpass"
        );
        assert_eq!(
            pgpass
                .find_password("localhost", 5432, "mydb", "myuser")
                .unwrap(),
            "mypass"
        );
    }
}
