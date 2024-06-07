use std::collections::HashSet;
use std::io::Read;
use std::path::{Path, PathBuf};

use crate::error::InnerError::{
    DifferentScriptNameTypes, InvalidSortMode, NotFound, NotSortableScriptNames, NotValidUtf8,
    PathParseError, UnknownPathType,
};
use crate::error::{ContextualError, ContextualResult};
use log::trace;
use nom::branch::alt;
use nom::bytes::complete::tag;
use nom::character::complete::{anychar, char, digit1};
use nom::combinator::{eof, map, map_res, recognize};
use nom::multi::{many0, many_till, separated_list1};
use nom::sequence::terminated;
use nom::{IResult, Parser};

use crate::script_discovery::script_filters::ScriptFilter;

/// ScripType is parsed from the first character of the script name,
/// where U is taken to mean undo, V is taken to mean Versioned, and
/// R is taken to mean Repeatable. Flyway operates with these
/// conventions.
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum ScriptType {
    Forward,
    Back,
    Repeatable,
}

/// VersionedName is a script name that normally has a version number
/// in it, and therefore should be run in a specific order. Some scripts
/// are [Repeatable](ScriptType::Repeatable) and do not have a version.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct VersionedName<'a> {
    whole_name: &'a str,
    version: Vec<u32>,
    name: &'a str,
    script_type: ScriptType,
}

fn parse_sql_file_name_til_eof(name: &str) -> IResult<&str, &str> {
    let suffix = ".sql";
    let (rem, name) = recognize(many_till(anychar, terminated(tag(suffix), eof)))(name)?;
    Ok((rem, &name[..name.len() - suffix.len()]))
}

fn parse_versioned_name_r(name: &str) -> IResult<&str, VersionedName> {
    let whole_name = name;
    let (name, script_type) =
        map(terminated(char('R'), tag("__")), |_| ScriptType::Repeatable)(name)?;
    let (rem, name) = parse_sql_file_name_til_eof(name)?;
    Ok((
        rem,
        VersionedName {
            whole_name,
            version: vec![],
            name,
            script_type,
        },
    ))
}

fn parse_versioned_name_uv(name: &str) -> IResult<&str, VersionedName> {
    let whole_name = name;
    let (name, script_type) = alt((
        char('V').map(|_| ScriptType::Forward),
        char('U').map(|_| ScriptType::Back),
    ))(name)?;
    let sep = alt((char('.'), char('_')));
    let version_part = map_res(digit1, |s: &str| s.parse::<u32>());
    let (name, version) = terminated(separated_list1(sep, version_part), tag("__"))(name)?;
    let (rem, name) = parse_sql_file_name_til_eof(name)?;
    Ok((
        rem,
        VersionedName {
            whole_name,
            version,
            name,
            script_type,
        },
    ))
}

/// Parses a flyway-style version script name.
///
/// There are two variants:
///   The name should begin with a V or U, followed by a version number, __ and then a name.
///   The name should begin with R, followed by __ and then a name.
///
/// The version number is a list of numbers separated by dots or underscores.
pub fn parse_versioned_name(name: &str) -> IResult<&str, VersionedName> {
    alt((parse_versioned_name_r, parse_versioned_name_uv))(name)
}

/// A script name that starts with a sequence number, which it should be sorted by.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct SequenceNumberName<'a> {
    whole_name: &'a str,
    sequence_number: u32,
    name: &'a str,
}

fn parse_sequence_number_name(name: &str) -> IResult<&str, SequenceNumberName> {
    let whole_name = name;
    let (name, sequence_number) = map_res(digit1, |s: &str| s.parse::<u32>())(name)?;
    let (name, _) = many0(char('_'))(name)?;
    let (rem, name) = parse_sql_file_name_til_eof(name)?;
    Ok((
        rem,
        SequenceNumberName {
            whole_name,
            sequence_number,
            name,
        },
    ))
}

/// A script that eugene can not sort by name, because there is no natural ordering in the name.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct SqlName<'a> {
    whole_name: &'a str,
    name: &'a str,
}

fn parse_sql_name(name: &str) -> IResult<&str, SqlName> {
    let whole_name = name;
    let (rem, name) = parse_sql_file_name_til_eof(name)?;
    Ok((rem, SqlName { whole_name, name }))
}

/// A best effort to parse a script name into something that can be sorted.
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum SqlScript<'a> {
    Versioned(VersionedName<'a>),
    SequenceNumber(SequenceNumberName<'a>),
    Sql(SqlName<'a>),
    Stdin,
}

impl PartialOrd for SqlScript<'_> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match (self, other) {
            (SqlScript::Versioned(left), SqlScript::Versioned(right))
                if !(matches!(left.script_type, ScriptType::Repeatable)
                    || matches!(right.script_type, ScriptType::Repeatable)) =>
            {
                left.version.partial_cmp(&right.version)
            }
            (SqlScript::SequenceNumber(left), SqlScript::SequenceNumber(right)) => {
                left.sequence_number.partial_cmp(&right.sequence_number)
            }
            _ => None,
        }
    }
}

/// The type of the script name. Use this to check that you are not trying to sort
/// incompatible script names, ie. filter out everything that can not be ordered.
#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub enum ScriptNameType {
    Versioned,
    Sequenced,
    None,
}

impl SqlScript<'_> {
    /// The title of the script name, without sequence number or version
    pub fn name(&self) -> &str {
        match self {
            SqlScript::Versioned(v) => v.name,
            SqlScript::SequenceNumber(v) => v.name,
            SqlScript::Sql(v) => v.name,
            SqlScript::Stdin => "stdin",
        }
    }
    /// The whole script name, including the version or sequence number and suffix
    pub fn whole_name(&self) -> &str {
        match self {
            SqlScript::Versioned(v) => v.whole_name,
            SqlScript::SequenceNumber(v) => v.whole_name,
            SqlScript::Sql(v) => v.whole_name,
            SqlScript::Stdin => "stdin",
        }
    }
    /// What type of name this is, to avoid sorting incompatible names together
    pub fn script_name_type(&self) -> ScriptNameType {
        match self {
            SqlScript::Versioned(_) => ScriptNameType::Versioned,
            SqlScript::SequenceNumber(_) => ScriptNameType::Sequenced,
            SqlScript::Sql(_) => ScriptNameType::None,
            SqlScript::Stdin => ScriptNameType::None,
        }
    }
}

fn parse_sql_script(name: &str) -> IResult<&str, SqlScript> {
    alt((
        map(parse_versioned_name, SqlScript::Versioned),
        map(parse_sequence_number_name, SqlScript::SequenceNumber),
        map(parse_sql_name, SqlScript::Sql),
    ))(name)
}

/// Discover the most likely naming scheme of a script and parse it into a [SqlScript]
pub fn parse(path: &Path) -> crate::Result<SqlScript> {
    let name = path
        .file_name()
        .ok_or_else(|| NotFound.with_context(format!("{path:?}")))?
        .to_str()
        .ok_or_else(|| NotValidUtf8.with_context(format!("{path:?}")))?;
    parse_sql_script(name)
        .map(|(_, script)| script)
        .map_err(|e| PathParseError.with_context(format!("{path:?} {e:?}")))
}

fn all_files_with_sql_suffix(dir: &Path) -> crate::Result<Vec<PathBuf>> {
    let mut entries = vec![];
    for entry in dir.read_dir().with_context(format!("{dir:?}"))? {
        let entry = entry?;
        let metadata = entry.metadata()?;
        if metadata.is_file() {
            let path = entry.path();
            if let Some(ext) = path.extension() {
                if ext == "sql" {
                    entries.push(entry.path());
                }
            }
        }
    }
    Ok(entries)
}

pub mod script_filters {
    use super::*;

    pub type ScriptFilter = fn(&SqlScript) -> bool;
    pub fn never(_: &SqlScript) -> bool {
        true
    }
    pub fn repatable_versioned(s: &SqlScript) -> bool {
        !matches!(s, SqlScript::Versioned(v) if v.script_type == ScriptType::Repeatable)
    }
    pub fn back(s: &SqlScript) -> bool {
        !matches!(s, SqlScript::Versioned(v) if v.script_type == ScriptType::Back)
    }
    pub fn skip_downgrade_and_repeatable(s: &SqlScript) -> bool {
        back(s) || repatable_versioned(s)
    }
}

fn sort_paths_by_script_type(
    paths: &[PathBuf],
    filter: ScriptFilter,
) -> crate::Result<Vec<PathBuf>> {
    let scripts: crate::Result<Vec<_>> = paths.iter().map(|p| Ok((p.clone(), parse(p)?))).collect();
    let mut scripts = scripts?;
    // Make some checks first, ensure that we can sort the paths,
    // they must all parse to something sortable of the same kind

    let script_types = scripts
        .iter()
        .map(|(_, s)| s.script_name_type())
        .collect::<HashSet<_>>();
    if script_types.len() > 1 {
        return Err(DifferentScriptNameTypes.with_context(format!(
            "Can not sort scripts of different types: {:?}",
            script_types
        )));
    }
    if script_types.contains(&ScriptNameType::None) {
        return Err(NotSortableScriptNames
            .with_context("Can not sort scripts without a sequence number or version"));
    }
    scripts.retain(|(_, s)| {
        let keep = filter(s);
        if !keep {
            trace!("Skipping: {:?}", s.whole_name());
        }
        keep
    });
    scripts.sort_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    Ok(scripts.into_iter().map(|(p, _)| p).collect())
}

/// Retrieves all SQL scripts from a folder and sorts them by their name
///
/// Errors if the folder does not exist, or if the scripts are of different types.
///
/// Sorting rules and discovered naming standards are:
///
/// - Versioned scripts are sorted by their version number according to flyway-like rules
/// - Scripts that start with a sequence number are sorted by that number (as an integer)
/// - Scripts that do not match any of the above are not sorted and an error is returned
pub fn sorted_migration_scripts_from_folder(
    dir: &Path,
    filter: ScriptFilter,
    sort: SortMode,
) -> crate::Result<Vec<PathBuf>> {
    let paths = all_files_with_sql_suffix(dir)?;
    match sort {
        SortMode::Auto => sort_paths_by_script_type(&paths, filter),
        SortMode::Unsorted => Ok(paths),
        SortMode::Lexicographic => {
            let mut paths = paths;
            paths.sort();
            Ok(paths)
        }
    }
}

/// A source for reading a SQL script from.
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum ReadFrom {
    Stdin,
    File(String),
    FileFromDirEntry(String),
}

impl ReadFrom {
    pub fn read(&self) -> crate::Result<String> {
        match self {
            ReadFrom::Stdin => {
                let mut buf = String::new();
                std::io::stdin()
                    .read_to_string(&mut buf)
                    .with_context("Failed to read stdin")?;
                Ok(buf)
            }
            ReadFrom::File(path) | ReadFrom::FileFromDirEntry(path) => {
                Ok(std::fs::read_to_string(path).with_context(format!("Failed to read {path}"))?)
            }
        }
    }
    pub fn name(&self) -> &str {
        match self {
            ReadFrom::Stdin => "stdin",
            ReadFrom::File(path) | ReadFrom::FileFromDirEntry(path) => path,
        }
    }
}
/// Discover scripts from a path, which can be a file or a directory, or -.
///
/// If the path is a directory, all files with the .sql suffix are discovered.
///
/// If the path is a file, it is returned as is. If the path is -, stdin is
/// returned. Otherwise, [SortMode] determines how the scripts are sorted.
pub fn discover_scripts(
    path: &str,
    filter: ScriptFilter,
    sort: SortMode,
) -> crate::Result<Vec<ReadFrom>> {
    if path == "-" {
        return Ok(vec![ReadFrom::Stdin]);
    }
    let data = std::fs::metadata(path)?;
    if data.is_file() {
        Ok(vec![ReadFrom::File(path.to_string())])
    } else if data.is_dir() {
        let paths = sorted_migration_scripts_from_folder(&PathBuf::from(path), filter, sort)?;
        Ok(paths
            .into_iter()
            .map(|p| ReadFrom::FileFromDirEntry(p.to_string_lossy().to_string()))
            .collect())
    } else {
        Err(UnknownPathType.with_context("Path is not a file or directory"))
    }
}

/// Discover scripts from `paths`, where each item can be a file or a directory, or -.
///
/// If the path is a directory, all files with the .sql suffix are discovered.
///
/// If the path is a file, it is returned as is. If the path is -, stdin is
/// returned.
pub fn discover_all<S: AsRef<str>, T: IntoIterator<Item = S>>(
    paths: T,
    filter: ScriptFilter,
    sort: SortMode,
) -> crate::Result<Vec<ReadFrom>> {
    let mut all = vec![];
    for path in paths {
        all.extend(discover_scripts(path.as_ref(), filter, SortMode::Unsorted)?);
    }

    let any_is_dir = all
        .iter()
        .any(|p| matches!(p, ReadFrom::FileFromDirEntry(_)));

    match sort {
        SortMode::Auto if any_is_dir || all.len() > 1 => {
            let all_paths: Vec<_> = all
                .into_iter()
                .map(|r| match r {
                    ReadFrom::File(p) | ReadFrom::FileFromDirEntry(p) => PathBuf::from(p),
                    ReadFrom::Stdin => PathBuf::from("stdin"),
                })
                .collect();
            for p in all_paths.iter() {
                trace!("Discovered: {:?}", p);
            }
            let all_paths = sort_paths_by_script_type(&all_paths, filter)?;
            for p in all_paths.iter() {
                trace!("Sorted: {:?}", p);
            }
            Ok(all_paths
                .into_iter()
                .map(|p| ReadFrom::File(p.to_string_lossy().to_string()))
                .collect())
        }
        SortMode::Lexicographic => {
            all.sort_by(|left, right| left.name().cmp(right.name()));
            Ok(all)
        }
        SortMode::Unsorted | SortMode::Auto => Ok(all),
    }
}

/// Which order to return discovered scripts from a folder
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum SortMode {
    /// Automatically determine the sorting mode by scanning the matching scripts
    ///
    /// We categorize the scripts into three groups:
    ///
    /// Versioned scripts:
    ///
    /// These either match `"[UV]_([0-9]+[_.])*([0-9]+)__[^.]+\.sql"` or `"R__[^.]+\.sql"`
    ///
    /// Sequenced scripts:
    ///
    /// These match `"[0-9]+_+[^.]+\.sql"`
    ///
    /// Unsorted scripts:
    ///
    /// When they are not versioned or sequenced
    ///
    /// If all scripts are versioned, they are sorted by version number. If all scripts are
    /// sequenced, they are sorted by sequence number. If there are unsorted scripts, an error
    /// is returned.
    Auto,
    /// Do not sort the scripts, return them in the order they were discovered
    Unsorted,
    /// Sort the scripts lexicographically by their name
    Lexicographic,
}

impl TryFrom<&str> for SortMode {
    type Error = crate::error::Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "auto" => Ok(SortMode::Auto),
            "none" => Ok(SortMode::Unsorted),
            "name" => Ok(SortMode::Lexicographic),
            _ => Err(InvalidSortMode.with_context(format!("Invalid sort mode: {}", value))),
        }
    }
}

impl TryFrom<String> for SortMode {
    type Error = crate::error::Error;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        SortMode::try_from(value.as_str())
    }
}

#[cfg(test)]
mod tests {
    use nom::error::ErrorKind::Eof;
    use pretty_assertions::assert_eq;
    use tempfile::TempDir;

    use super::*;

    #[test]
    fn parses_versioned_names() {
        let expect = VersionedName {
            whole_name: "V1_2__create_table.sql",
            version: vec![1, 2],
            name: "create_table",
            script_type: ScriptType::Forward,
        };
        assert_eq!(parse_versioned_name(expect.whole_name).unwrap().1, expect);
        let expect = VersionedName {
            whole_name: "U1_2__drop_table.sql",
            version: vec![1, 2],
            name: "drop_table",
            script_type: ScriptType::Back,
        };
        assert_eq!(parse_versioned_name(expect.whole_name).unwrap().1, expect);
        let expect = VersionedName {
            whole_name: "R__create_table.sql",
            version: vec![],
            name: "create_table",
            script_type: ScriptType::Repeatable,
        };
        let whole_name = "R_1_2_3__create_table.sql"; // not valid
        assert_eq!(parse_versioned_name(expect.whole_name).unwrap().1, expect);
        let expect = Err(nom::Err::Error(nom::error::Error {
            input: whole_name,
            code: nom::error::ErrorKind::Char,
        }));
        assert_eq!(parse_versioned_name(whole_name), expect);
        let expect = Err(nom::Err::Error(nom::error::Error {
            input: "T1__create_table.old.sql",
            code: nom::error::ErrorKind::Char,
        }));
        assert_eq!(parse_versioned_name("T1__create_table.old.sql"), expect);
    }

    #[test]
    fn parses_sequence_number_name() {
        let expect = SequenceNumberName {
            whole_name: "1__create_table.sql",
            sequence_number: 1,
            name: "create_table",
        };
        assert_eq!(
            parse_sequence_number_name(expect.whole_name).unwrap().1,
            expect
        );
        let expect = SequenceNumberName {
            whole_name: "2__drop_table.sql",
            sequence_number: 2,
            name: "drop_table",
        };
        assert_eq!(
            parse_sequence_number_name(expect.whole_name).unwrap().1,
            expect
        );
        let expect = Err(nom::Err::Error(nom::error::Error {
            input: "T1__create_table.old.sql",
            code: nom::error::ErrorKind::Digit,
        }));
        assert_eq!(
            parse_sequence_number_name("T1__create_table.old.sql"),
            expect
        );
        let name = "1.sql";
        let expect = SequenceNumberName {
            whole_name: name,
            sequence_number: 1,
            name: "",
        };
        assert_eq!(parse_sequence_number_name(name).unwrap().1, expect);
    }

    #[test]
    fn parses_sql_name() {
        let expect = SqlName {
            whole_name: "create_table.sql",
            name: "create_table",
        };
        assert_eq!(parse_sql_name(expect.whole_name).unwrap().1, expect,);
        let expect = SqlName {
            whole_name: "drop_table.sql",
            name: "drop_table",
        };
        assert_eq!(parse_sql_name(expect.whole_name).unwrap().1, expect,);
        let expect = Err(nom::Err::Error(nom::error::Error {
            input: "",
            code: Eof,
        }));
        assert_eq!(parse_sql_name("create_table.old.xlsx"), expect,);
    }

    #[test]
    fn parses_sql_script() {
        let versioned = SqlScript::Versioned(VersionedName {
            whole_name: "V1_2__create_table.sql",
            version: vec![1, 2],
            name: "create_table",
            script_type: ScriptType::Forward,
        });
        let numbered = SqlScript::SequenceNumber(SequenceNumberName {
            whole_name: "1__create_table.sql",
            sequence_number: 1,
            name: "create_table",
        });
        let sql = SqlScript::Sql(SqlName {
            whole_name: "create_table.sql",
            name: "create_table",
        });
        assert_eq!(
            Ok(("", versioned.clone())),
            parse_sql_script(versioned.whole_name())
        );
        assert_eq!(
            Ok(("", numbered.clone())),
            parse_sql_script(numbered.whole_name())
        );
        assert_eq!(Ok(("", sql.clone())), parse_sql_script(sql.whole_name()));
    }

    #[test]
    fn test_sorted_mixed_types_errors() {
        let paths = vec![
            PathBuf::from("1__create_table.sql"),
            PathBuf::from("V1_2__create_table.sql"),
        ];
        let res = sort_paths_by_script_type(&paths, script_filters::never);
        assert!(res.is_err());
        let paths = vec![
            PathBuf::from("1_create_table.sql"),
            PathBuf::from("create_table.sql"),
        ];
        let res = sort_paths_by_script_type(&paths, script_filters::never);
        assert!(res.is_err());
        let paths = vec![
            PathBuf::from("1__create_table.sql"),
            PathBuf::from("R__create_table.sql"),
        ];
        let res = sort_paths_by_script_type(&paths, script_filters::never);
        assert!(res.is_err());
    }

    #[test]
    fn can_remove_repeatable_scripts() {
        let paths = vec![
            PathBuf::from("V1__create_table.sql"),
            PathBuf::from("R__create_table.sql"),
        ];
        let res = sort_paths_by_script_type(&paths, script_filters::repatable_versioned);
        assert_eq!(res.unwrap(), vec![PathBuf::from("V1__create_table.sql")]);
    }

    #[test]
    fn can_remove_downgrades() {
        let paths = vec![
            PathBuf::from("V1__create_table.sql"),
            PathBuf::from("U1__create_table.sql"),
        ];
        let res = sort_paths_by_script_type(&paths, script_filters::back);
        assert_eq!(res.unwrap(), vec![PathBuf::from("V1__create_table.sql")]);
    }

    #[test]
    fn check_all_examples_sort() {
        let examples_dir = "examples";
        // Iterate over every subdirectory in the examples directory
        for entry in std::fs::read_dir(examples_dir).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.is_dir() {
                // Iterate over every subdirectory in path
                for entry in std::fs::read_dir(&path).unwrap() {
                    let entry = entry.unwrap();
                    let path = entry.path();
                    if path.is_dir() {
                        assert!(sorted_migration_scripts_from_folder(
                            &path,
                            script_filters::never,
                            SortMode::Auto
                        )
                        .is_ok());
                    }
                }
            }
        }
    }

    #[test]
    fn sorts_like_flyway() {
        // make a temporary directory
        let temp_dir = TempDir::new().unwrap();
        let p = temp_dir.path();
        let a_files = vec!["V2__foo.sql", "V3__bar.sql"];
        let b_files = vec!["V1__foo.sql", "V4__bar.sql"];
        let a_dir = p.join("a");
        let b_dir = p.join("b");
        std::fs::create_dir_all(&a_dir).unwrap();
        std::fs::create_dir_all(&b_dir).unwrap();
        for file in a_files {
            std::fs::write(a_dir.join(file), "").unwrap();
        }
        for file in b_files {
            std::fs::write(b_dir.join(file), "").unwrap();
        }
        let all = discover_all(
            vec![a_dir.to_string_lossy(), b_dir.to_string_lossy()],
            script_filters::never,
            SortMode::Auto,
        )
        .unwrap();
        let names: Vec<PathBuf> = all.iter().map(|r| r.name().into()).collect::<Vec<_>>();
        assert_eq!(
            names,
            vec![
                b_dir.join("V1__foo.sql"),
                a_dir.join("V2__foo.sql"),
                a_dir.join("V3__bar.sql"),
                b_dir.join("V4__bar.sql")
            ]
        );
    }
}
