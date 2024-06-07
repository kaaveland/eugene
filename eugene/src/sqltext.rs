use crate::error::InnerError::UnresolvedPlaceHolder;
use crate::error::{ContextualError, ContextualResult};
use std::collections::HashMap;
use std::io::Read;

/// Naively resolve placeholders in SQL script in ${} format using provided mapping
pub fn resolve_placeholders(sql: &str, mapping: &HashMap<&str, &str>) -> crate::Result<String> {
    let placeholder_re = regex::Regex::new(r"\$\{[a-zA-Z0-9]+}").unwrap();
    let resolved = mapping.iter().fold(sql.to_string(), |acc, (k, v)| {
        acc.replace(&format!("${{{}}}", k), v)
    });
    if let Some(m) = placeholder_re.find(&resolved) {
        Err(UnresolvedPlaceHolder
            .with_context(format!("Unresolved placeholder: {}", m.to_owned().as_str())))
    } else {
        Ok(resolved)
    }
}

/// Separate SQL script into statements
pub fn sql_statements(sql: &str) -> crate::Result<Vec<&str>> {
    Ok(pg_query::split_with_parser(sql)?
        .into_iter()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect())
}

/// This function reads SQL script files, discards comments and returns a vector of
/// strings containing SQL statements.
pub fn read_sql_statements(path: &str) -> crate::Result<String> {
    if path == "-" {
        let mut buffer = String::new();
        std::io::stdin().read_to_string(&mut buffer)?;
        Ok(buffer)
    } else {
        std::fs::read_to_string(path).with_context(format!("Failed to read file: {}", path))
    }
}

/// Check if a SQL statement is a CREATE INDEX CONCURRENTLY statement or similar, which
/// must run outside of a transaction.
pub fn is_concurrently<S: AsRef<str>>(sql: S) -> bool {
    let sql = sql.as_ref();
    sql.to_lowercase().contains("concurrently")
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    #[test]
    fn test_split_statements_with_comments() -> crate::Result<()> {
        let sql = "SELECT * FROM tab; -- This is a comment\nSELECT * FROM tab; /* This is a block comment */";
        let result = super::sql_statements(sql)?;
        assert_eq!(
            result,
            vec![
                "SELECT * FROM tab",
                "-- This is a comment\nSELECT * FROM tab"
            ]
        );
        Ok(())
    }

    #[test]
    fn test_split_with_dollars() -> crate::Result<()> {
        let s = "CREATE OR REPLACE FUNCTION test_fn(rolename NAME) RETURNS TEXT AS
$$
BEGIN
  RETURN 1
END;
$$
LANGUAGE plpgsql; select * from tab";
        let result = super::sql_statements(s)?;
        assert_eq!(
            result,
            vec![
                &s[..s.len() - 1 - " select * from tab".len()],
                "select * from tab"
            ]
        );
        Ok(())
    }

    #[test]
    fn test_split_with_dollars_body() {
        let s = "CREATE OR REPLACE FUNCTION get_employee_details(emp_id INT)
RETURNS TABLE (
    employee_id INT,
    employee_name VARCHAR,
    department VARCHAR,
    salary NUMERIC
) AS $body$
BEGIN
    RETURN QUERY
    SELECT
        e.id AS employee_id,
        e.name AS employee_name,
        d.name AS department,
        e.salary
    FROM
        employees e
    JOIN
        departments d ON e.department_id = d.id
    WHERE
        e.id = emp_id;
END;
$body$ LANGUAGE plpgsql;"; // generated example by ai
        let result = super::sql_statements(s).unwrap();
        assert_eq!(result, vec![&s[..s.len() - 1]]);
    }
}
