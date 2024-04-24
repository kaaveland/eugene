use anyhow::Result;
use std::collections::HashMap;
use std::io::{Error, Read};

/// Naively resolve placeholders in SQL script in ${} format using provided mapping
pub fn resolve_placeholders(sql: &str, mapping: &HashMap<&str, &str>) -> Result<String> {
    let placeholder_re = regex::Regex::new(r"\$\{[a-zA-Z0-9]+}").unwrap();
    let resolved = mapping.iter().fold(sql.to_string(), |acc, (k, v)| {
        acc.replace(&format!("${{{}}}", k), v)
    });
    if let Some(m) = placeholder_re.find(&resolved) {
        return Err(anyhow::anyhow!(
            "Unresolved placeholder: {}",
            m.to_owned().as_str()
        ));
    } else {
        Ok(resolved)
    }
}

/// Strip comments from SQL scripts provided as str
pub fn strip_comments(sql: &str) -> String {
    let mut content = sql.chars().peekable();
    let mut result = String::new();

    while let Some(c) = content.next() {
        let next = content.peek().copied();
        match (c, next) {
            ('-', Some('-')) => {
                while let Some(_) = content.next() {
                    if content.peek().copied() == Some('\n') {
                        break;
                    }
                }
            }
            ('/', Some('*')) => {
                content.next();
                while let Some(c) = content.next() {
                    if c == '*' && content.peek().copied() == Some('/') {
                        content.next();
                        break;
                    }
                }
            }
            (ch, _) => {
                result.push(ch);
            }
        }
    }

    result
}

/// Separate SQL script into statements after stripping comments.
/// Statements are separated by semicolons, although if we find a $$ we must scan to the matching one.
pub fn sql_statements(sql: &str) -> Vec<String> {
    let sql = strip_comments(sql);
    let mut content = sql.chars().peekable();
    let mut result = Vec::new();
    let mut statement = String::new();
    let mut in_string = false;
    while let Some(c) = content.next() {
        let next = content.peek().copied();
        statement.push(c);
        match (c, next) {
            ('$', Some('$')) if !in_string => {
                // Scan until the next $$
                statement.push(content.next().unwrap());
                while let Some(c) = content.next() {
                    statement.push(c);
                    if c == '$' && content.peek().copied() == Some('$') {
                        statement.push(content.next().unwrap());
                        break;
                    }
                }
            }
            (';', _) if !in_string => {
                result.push(statement);
                statement = String::new();
            }
            ('\'', _) => {
                in_string = !in_string;
            }
            _ => {}
        }
    }
    if !statement.is_empty() {
        result.push(statement);
    }
    result.retain(|s| !s.trim().is_empty());
    result
}

/// This function reads SQL script files, discards comments and returns a vector of
/// strings containing SQL statements.
pub fn read_sql_statements(path: &str) -> Result<String, Error> {
    if path == "-" {
        let mut buffer = String::new();
        std::io::stdin().read_to_string(&mut buffer)?;
        Ok(buffer)
    } else {
        std::fs::read_to_string(path)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_strip_comments() {
        let sql = "SELECT * FROM table; -- This is a comment";
        let result = super::strip_comments(sql);
        assert_eq!(result, "SELECT * FROM table; ");
    }
    #[test]
    fn test_strip_block_comments() {
        let sql = "SELECT * FROM table; /* This is a block comment */";
        let result = super::strip_comments(sql);
        assert_eq!(result, "SELECT * FROM table; ");
    }
    #[test]
    fn test_strip_mixed_comments_two_statements() {
        let sql = "SELECT * FROM table; -- This is a comment\nSELECT * FROM table; /* This is a block comment */";
        let result = super::strip_comments(sql);
        assert_eq!(result, "SELECT * FROM table; \nSELECT * FROM table; ");
    }
    #[test]
    fn test_split_statements_with_comments() {
        let sql = "SELECT * FROM table; -- This is a comment\nSELECT * FROM table; /* This is a block comment */";
        let result = super::sql_statements(sql);
        assert_eq!(result, vec!["SELECT * FROM table", "SELECT * FROM table"]);
    }
}
