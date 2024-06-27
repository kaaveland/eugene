use std::collections::HashMap;
use std::io::Read;

use nom::branch::alt;
use nom::bytes::complete::tag;
use nom::character::complete::{anychar, multispace0, multispace1};
use nom::combinator::recognize;
use nom::multi::{many0, many_till};
use nom::sequence::pair;
use nom::IResult;

use crate::error::InnerError::UnresolvedPlaceHolder;
use crate::error::{ContextualError, ContextualResult};

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

fn parse_line_comment(s: &str) -> IResult<&str, &str> {
    let (s, _) = pair(multispace0, tag("--"))(s)?;
    let (s, _) = many_till(anychar, tag("\n"))(s)?;
    Ok((s, ""))
}

fn parse_comment_block(s: &str) -> IResult<&str, &str> {
    let (s, _) = pair(multispace0, tag("/*"))(s)?;
    let (s, _) = many_till(anychar, tag("*/"))(s)?;
    Ok((s, ""))
}

fn parse_blanks_and_comments(s: &str) -> IResult<&str, &str> {
    let (s, pre) = recognize(many0(alt((
        multispace1,
        parse_line_comment,
        parse_comment_block,
    ))))(s)?;
    Ok((s, pre))
}

/// Discover which line within possibly multiline statement that the actual statement starts on
fn line_no_of_start(statement: &str) -> crate::Result<usize> {
    if let Ok((_, pre)) = parse_blanks_and_comments(statement) {
        Ok(pre.chars().filter(|&c| c == '\n').count())
    } else {
        Ok(0usize)
    }
}

/// Split into statements along with the line number where each statement starts, skipping leading blanks and comments
pub fn sql_statements_with_line_no(sql: &str) -> crate::Result<Vec<(usize, &str)>> {
    let numbered_statements: crate::Result<Vec<_>> = pg_query::split_with_parser(sql)?
        .into_iter()
        .map(|s| Ok((line_no_of_start(s)?, s)))
        .collect();
    let mut numbered_statements = numbered_statements?;
    let mut line = 1;
    for st in numbered_statements.iter_mut() {
        st.0 += line;
        line += st.1.chars().filter(|&c| c == '\n').count();
        st.1 = st.1.trim();
    }
    Ok(numbered_statements)
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
    fn number_those_lines_ex1() {
        let ex = "ALTER TABLE foo ADD a text;


-- A comment
CREATE UNIQUE INDEX my_index ON foo (a);";
        let result = super::sql_statements_with_line_no(ex).unwrap();
        assert_eq!(result[0].0, 1);
        assert_eq!(result[1].0, 5);
    }

    #[test]
    fn number_those_lines_ex2() {
        let ex = "ALTER TABLE
    foo
ADD
    a text;

CREATE UNIQUE INDEX
    my_index ON foo (a);";
        let result = super::sql_statements_with_line_no(ex).unwrap();
        assert_eq!(result[0].0, 1);
        assert_eq!(result[1].0, 6);
    }

    #[test]
    fn number_those_lines_ex3() {
        let ex = "CREATE TABLE AUTHORS (
    ID INT GENERATED ALWAYS AS IDENTITY
        PRIMARY KEY,
    NAME TEXT
);

ALTER TABLE BOOKS
    ADD COLUMN AUTHOR_ID INT;

ALTER TABLE BOOKS
    ADD CONSTRAINT AUTHOR_ID_FK
        FOREIGN KEY (AUTHOR_ID)
        REFERENCES AUTHORS (ID);";
        let result = super::sql_statements_with_line_no(ex).unwrap();
        assert_eq!(result[0].0, 1);
        assert_eq!(result[1].0, 7);
        assert_eq!(result[2].0, 10);
    }

    #[test]
    fn test_split_statements_with_comments() -> crate::Result<()> {
        let sql = "SELECT * FROM tab; -- This is a comment\nSELECT * FROM tab; /* This is a block comment */";
        let result = super::sql_statements_with_line_no(sql)?;
        assert_eq!(
            result,
            vec![
                (1, "SELECT * FROM tab"),
                (2, "-- This is a comment\nSELECT * FROM tab")
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
        let result = super::sql_statements_with_line_no(s)?;
        assert_eq!(
            result,
            vec![
                (1, &s[..s.len() - 1 - " select * from tab".len()]),
                (7, "select * from tab")
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
        let result = super::sql_statements_with_line_no(s).unwrap();
        assert_eq!(result, vec![(1, &s[..s.len() - 1])]);
    }

    #[test]
    fn parses_blanks_line_comments() {
        let s = "  \n--comment\nsqltext";
        let result = super::parse_blanks_and_comments(s);
        assert_eq!(result.unwrap(), ("sqltext", "  \n--comment\n"));
    }

    #[test]
    fn parses_comment_blocks() {
        let s = "  /*comment\n\n*/sqltext";
        let result = super::parse_blanks_and_comments(s);
        assert_eq!(result.unwrap(), ("sqltext", "  /*comment\n\n*/"));
    }
}
