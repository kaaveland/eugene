use crate::error::InnerError;
use nom::branch::alt;
use nom::bytes::complete::tag;
use nom::character::complete::{anychar, line_ending, multispace0, satisfy, space0};
use nom::combinator::{eof, map, peek, recognize};
use nom::multi::many_till;
use nom::sequence::{delimited, preceded, terminated};
use nom::IResult;

fn open_line_comment(s: &str) -> IResult<&str, &str> {
    terminated(tag("--"), space0)(s)
}

fn close_line_comment_or_eof(s: &str) -> IResult<&str, &str> {
    terminated(space0, alt((line_ending, eof)))(s)
}

fn file_name_char(c: char) -> bool {
    !c.is_whitespace() && c != ';' && c != ':'
}

fn ends_with_dot_sql(s: &str) -> IResult<&str, &str> {
    terminated(tag(".sql"), close_line_comment_or_eof)(s)
}

fn file_name(s: &str) -> IResult<&str, &str> {
    let (s, mut n) = recognize(many_till(satisfy(file_name_char), ends_with_dot_sql))(s)?;
    while let Some(c) = n.chars().last() {
        if c.is_whitespace() {
            n = &n[..n.len() - 1];
        } else {
            break;
        }
    }
    Ok((s, n))
}

fn file_colon(s: &str) -> IResult<&str, &str> {
    terminated(tag("file:"), space0)(s)
}

fn file_comment(s: &str) -> IResult<&str, &str> {
    let (s, _) = open_line_comment(s)?;
    let mut choice = alt((preceded(file_colon, file_name), file_name));
    let (s, file_name) = choice(s)?;
    Ok((s, file_name))
}

fn until_file_comment(s: &str) -> IResult<&str, &str> {
    recognize(many_till(anychar, alt((eof, peek(file_comment)))))(s)
}

fn file_comment_and_sql(s: &str) -> IResult<&str, (&str, &str)> {
    let (s, comment) = file_comment(s)?;
    let (s, sql) = until_file_comment(s)?;
    Ok((s, (sql, comment)))
}

fn script_section(s: &str) -> IResult<&str, (&str, &str)> {
    let (s, section) = recognize(file_comment_and_sql)(s)?;
    let (_, name) = file_comment(section)?;
    Ok((s, (name, section)))
}

pub fn break_into_files(s: &str) -> crate::Result<Vec<(Option<&str>, &str)>> {
    let each = delimited(
        multispace0,
        alt((
            map(script_section, |(name, sql)| (Some(name), sql)),
            map(until_file_comment, |sql| (None, sql)),
        )),
        multispace0,
    );

    many_till(each, eof)(s)
        .map(|(_, (files, _))| files)
        .map_err(|e| InnerError::ScriptParsingError(format!("{e:?}")).into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn parses_named_script_section() {
        let input = "-- file: foo.sql\nSELECT * FROM foo;";
        let result = script_section(input);
        assert_eq!(
            result,
            Ok(("", ("foo.sql", "-- file: foo.sql\nSELECT * FROM foo;")))
        );
    }

    #[test]
    fn parses_file_comment_and_sql() {
        let input = "-- file: foo.sql\nSELECT * FROM foo;";
        let result = file_comment_and_sql(input);
        assert_eq!(result, Ok(("", ("SELECT * FROM foo;", "foo.sql"))));
    }

    #[test]
    fn parses_until_file_comment() {
        let input = "select * from books; -- file: foo.sql";
        let result = until_file_comment(input);
        assert_eq!(result, Ok(("-- file: foo.sql", "select * from books; ")));
    }

    #[test]
    fn file_comment_examples() {
        let input = "-- foo.sql\nSELECT * FROM foo;";
        let result = file_comment(input);
        assert_eq!(result, Ok(("SELECT * FROM foo;", "foo.sql")));
        let input = "-- file:foo.sql\nSELECT * FROM foo;";
        let result = file_comment(input);
        assert_eq!(result, Ok(("SELECT * FROM foo;", "foo.sql")));
        let input = "-- file: foo.sql\nSELECT * FROM foo;";
        let result = file_comment(input);
        assert_eq!(result, Ok(("SELECT * FROM foo;", "foo.sql")));
    }

    #[test]
    fn when_input_has_no_file_comments() {
        let input = "SELECT * FROM foo;";
        let result = break_into_files(input).unwrap();
        assert_eq!(result, vec![(None, input)]);
    }

    #[test]
    fn when_input_has_single_file_comment() {
        let input = "-- foo.sql\nSELECT * FROM foo;";
        let result = break_into_files(input).unwrap();
        assert_eq!(
            result,
            vec![(Some("foo.sql"), "-- foo.sql\nSELECT * FROM foo;")]
        );
    }

    #[test]
    fn when_input_has_two_file_comments() {
        let input = "-- foo.sql\nSELECT * FROM foo;\n-- bar.sql\nSELECT * FROM bar;";
        let result = break_into_files(input).unwrap();
        assert_eq!(
            result,
            vec![
                (Some("foo.sql"), "-- foo.sql\nSELECT * FROM foo;\n"),
                (Some("bar.sql"), "-- bar.sql\nSELECT * FROM bar;")
            ]
        );
    }

    #[test]
    fn when_input_has_no_file_comment_but_other_comments() {
        let input = "-- eugene: ignore E3\nSELECT * FROM foo;";
        let result = break_into_files(input).unwrap();
        assert_eq!(result, vec![(None, input)]);
    }

    #[test]
    fn example_from_eugene_doc() {
        let example = "-- 1.sql

create table authors (
    id integer generated always as identity
        primary key,
    name text not null
);

-- 2.sql

set local lock_timeout = '2s';
alter table authors
    add column email text not null;
select count(*) from authors;
";
        let result = break_into_files(example).unwrap();
        let names = result.iter().map(|(name, _)| name).collect::<Vec<_>>();
        assert_eq!(names, vec![&Some("1.sql"), &Some("2.sql")]);
    }
}
