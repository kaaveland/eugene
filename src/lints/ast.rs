use anyhow::Context;
use pg_query::protobuf::{
    AlterTableCmd, AlterTableType, ColumnDef, ConstrType, CreateStmt, CreateTableAsStmt, IndexStmt,
    VariableSetStmt,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColDefSummary {
    pub name: String,
    pub type_name: String,
}
/// A simpler, linter-rule friendly representation of the postgres parse tree
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StatementSummary {
    Ignored,
    LockTimeout,
    CreateTable {
        schema: String,
        name: String,
        columns: Vec<ColDefSummary>,
    },
    CreateTableAs {
        schema: String,
        name: String,
    },
    CreateIndex {
        schema: String,
        idxname: String,
        concurrently: bool,
        target: String,
    },
    AlterTable {
        schema: String,
        name: String,
        actions: Vec<AlterTableAction>,
    },
}

impl StatementSummary {
    /// Returns a list of (schema, name) tuples for objects created by this statement
    pub fn created_objects(&self) -> Vec<(&str, &str)> {
        match self {
            StatementSummary::CreateIndex {
                schema, idxname, ..
            } => vec![(schema, idxname)],
            StatementSummary::CreateTable { schema, name, .. } => vec![(schema, name)],
            StatementSummary::CreateTableAs { schema, name } => vec![(schema, name)],
            StatementSummary::Ignored
            | StatementSummary::LockTimeout
            | StatementSummary::AlterTable { .. } => {
                vec![]
            }
        }
    }
    /// Returns a list of (schema, name) tuples for objects locked by this statement
    ///
    /// For CREATE INDEX, the index and the table/matview are both locked
    pub fn lock_targets(&self) -> Vec<(&str, &str)> {
        match self {
            StatementSummary::CreateIndex { concurrently, .. } if *concurrently => vec![],
            StatementSummary::CreateIndex { schema, target, .. } => vec![(schema, target)],
            StatementSummary::CreateTable { .. } | StatementSummary::CreateTableAs { .. } => vec![],
            StatementSummary::AlterTable { schema, name, .. } => vec![(schema, name)],
            StatementSummary::Ignored | StatementSummary::LockTimeout => vec![],
        }
    }
}

/// Represents an action taken in an ALTER TABLE statement, such as setting a column type
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AlterTableAction {
    SetType {
        column: String,
        type_name: String,
    },
    SetNotNull {
        column: String,
    },
    AddConstraint {
        name: String,
        use_index: bool,
        constraint_type: ConstrType,
        valid: bool,
    },
    AddColumn {
        column: String,
        type_name: String,
    },
    Unrecognized,
}

fn set_statement(child: &VariableSetStmt) -> anyhow::Result<StatementSummary> {
    if child.name.eq_ignore_ascii_case("lock_timeout") {
        Ok(StatementSummary::LockTimeout)
    } else {
        Ok(StatementSummary::Ignored)
    }
}

fn create_table(child: &CreateStmt) -> anyhow::Result<StatementSummary> {
    if let Some(rel) = &child.relation {
        let schema = rel.schemaname.clone();
        let name = rel.relname.clone();
        let elts: anyhow::Result<Vec<_>> = child
            .table_elts
            .iter()
            .map(|node| {
                let inner = node.node.as_ref().map(|node| node.to_ref());
                if let Some(pg_query::NodeRef::ColumnDef(coldef)) = inner {
                    let name = coldef.colname.clone();
                    let type_name = col_type_as_string(coldef)?;
                    Ok(ColDefSummary { name, type_name })
                } else {
                    Err(anyhow::anyhow!(
                        "CREATE TABLE statement has an unrecognized column definition"
                    ))
                }
            })
            .collect();
        Ok(StatementSummary::CreateTable {
            schema,
            name,
            columns: elts?,
        })
    } else {
        Err(anyhow::anyhow!(
            "CREATE TABLE statement does not have a relation"
        ))
    }
}

fn create_table_as(child: &CreateTableAsStmt) -> anyhow::Result<StatementSummary> {
    if let Some(dest) = &child.into {
        if let Some(rel) = &dest.rel {
            let schema = rel.schemaname.clone();
            let name = rel.relname.clone();
            Ok(StatementSummary::CreateTableAs { schema, name })
        } else {
            Err(anyhow::anyhow!(
                "CREATE TABLE AS statement does not have a relation"
            ))
        }
    } else {
        Err(anyhow::anyhow!(
            "CREATE TABLE AS statement does not have a destination"
        ))
    }
}

fn create_index(child: &IndexStmt) -> anyhow::Result<StatementSummary> {
    if let Some(rel) = &child.relation {
        let schema = rel.schemaname.clone();
        let idxname = child.idxname.clone();
        Ok(StatementSummary::CreateIndex {
            concurrently: child.concurrent,
            target: rel.relname.to_string(),
            schema,
            idxname,
        })
    } else {
        Err(anyhow::anyhow!(
            "CREATE INDEX statement does not have a relation"
        ))
    }
}

fn col_type_as_string(coldef: &ColumnDef) -> anyhow::Result<String> {
    if let Some(tp) = &coldef.type_name {
        let names: anyhow::Result<Vec<String>> = tp
            .names
            .iter()
            .map(|n| match n.node.as_ref() {
                Some(pg_query::protobuf::node::Node::String(tn)) => Ok(tn.sval.to_owned()),
                _ => Err(anyhow::anyhow!("Column definition has no type name")),
            })
            .collect();
        Ok(names?.join("."))
    } else {
        Err(anyhow::anyhow!("Column definition has no type name"))
    }
}

fn parse_alter_table_action(child: &AlterTableCmd) -> anyhow::Result<AlterTableAction> {
    let subtype = AlterTableType::from_i32(child.subtype)
        .context(format!("Invalid AlterTableCmd subtype: {}", child.subtype))?;
    match subtype {
        AlterTableType::AtAlterColumnType => {
            let col = expect_coldef(child)?;
            // TODO: Parse the type name
            Ok(AlterTableAction::SetType {
                column: child.name.clone(),
                type_name: col_type_as_string(col)?,
            })
        }
        AlterTableType::AtAddColumn => {
            let col = expect_coldef(child)?;
            Ok(AlterTableAction::AddColumn {
                column: col.colname.clone(),
                type_name: col_type_as_string(col)?,
            })
        }
        AlterTableType::AtSetNotNull => Ok(AlterTableAction::SetNotNull {
            column: child.name.clone(),
        }),
        AlterTableType::AtAddConstraint => {
            let def = expect_constraint_def(child)?;
            let name = def.conname.clone();

            let constraint_type = def.contype;
            let constraint_type = ConstrType::from_i32(constraint_type)
                .context(format!("Invalid constraint type: {}", constraint_type))?;
            let use_index = !def.indexname.is_empty();
            let valid = !def.skip_validation;
            Ok(AlterTableAction::AddConstraint {
                name,
                use_index,
                constraint_type,
                valid,
            })
        }
        _ => Ok(AlterTableAction::Unrecognized),
    }
}

fn expect_constraint_def(child: &AlterTableCmd) -> anyhow::Result<&pg_query::protobuf::Constraint> {
    if let Some(def) = &child.def {
        let next = def.node.as_ref();
        if let Some(n) = next {
            if let pg_query::NodeRef::Constraint(constraint) = n.to_ref() {
                Ok(constraint)
            } else {
                Err(anyhow::anyhow!(
                    "AlterTableCmd Expected constraint def, found: {n:?}"
                ))
            }
        } else {
            Err(anyhow::anyhow!(
                "AlterTableCmd expected constraint def node, found none"
            ))
        }
    } else {
        Err(anyhow::anyhow!(
            "AlterTableCmd expected constraint def, found none"
        ))
    }
}

fn expect_coldef(child: &AlterTableCmd) -> anyhow::Result<&ColumnDef> {
    if let Some(def) = &child.def {
        let next = def.node.as_ref();
        if let Some(n) = next {
            if let pg_query::NodeRef::ColumnDef(colddef) = n.to_ref() {
                Ok(colddef)
            } else {
                Err(anyhow::anyhow!(
                    "AlterTableCmd Expected column def, found: {n:?}"
                ))
            }
        } else {
            Err(anyhow::anyhow!(
                "AlterTableCmd expected column def node, found none"
            ))
        }
    } else {
        Err(anyhow::anyhow!(
            "AlterTableCmd expected column def, found none"
        ))
    }
}

fn alter_table(child: &pg_query::protobuf::AlterTableStmt) -> anyhow::Result<StatementSummary> {
    if let Some(rel) = &child.relation {
        let schema = rel.schemaname.clone();
        let name = rel.relname.clone();
        let actions: anyhow::Result<Vec<_>> = child
            .cmds
            .iter()
            .map(|cmd| {
                if let Some(cmd_node) = &cmd.node {
                    let node_ref = &cmd_node.to_ref();
                    if let pg_query::NodeRef::AlterTableCmd(child) = node_ref {
                        parse_alter_table_action(child)
                    } else {
                        Err(anyhow::anyhow!(
                            "ALTER TABLE statement has an unrecognized command node: {node_ref:?}"
                        ))
                    }
                } else {
                    Err(anyhow::anyhow!("ALTER TABLE statement has no command node"))
                }
            })
            .collect();
        Ok(StatementSummary::AlterTable {
            schema,
            name,
            actions: actions?,
        })
    } else {
        Err(anyhow::anyhow!(
            "ALTER TABLE statement does not have a relation"
        ))
    }
}

/// Describes a statement in a linter-friendly way by simplifying the parse tree
///
/// Will return `Ok(StatementSummary::Ignored)` if the statement is not recognized
///
/// # Errors
///
/// If the parse tree has an unexpected structure, an error can be returned. This could be for example,
/// a parse tree that represents an `alter column set type` command, but without a new type declaration.
pub fn describe(statement: &pg_query::NodeRef) -> anyhow::Result<StatementSummary> {
    match statement {
        pg_query::NodeRef::VariableSetStmt(child) => set_statement(child),
        // CREATE TABLE
        pg_query::NodeRef::CreateStmt(child) => create_table(child),
        // CREATE TABLE AS
        pg_query::NodeRef::CreateTableAsStmt(child) => create_table_as(child),
        // CREATE INDEX
        pg_query::NodeRef::IndexStmt(child) => create_index(child),
        pg_query::NodeRef::AlterTableStmt(child) => alter_table(child),
        _ => Ok(StatementSummary::Ignored),
    }
}

#[cfg(test)]
mod tests {
    use crate::lints::StatementSummary;

    fn parse_s(s: &str) -> StatementSummary {
        super::describe(
            &pg_query::parse(s).unwrap().protobuf.stmts[0]
                .stmt
                .as_ref()
                .unwrap()
                .node
                .as_ref()
                .unwrap()
                .to_ref(),
        )
        .unwrap()
    }

    #[test]
    fn test_set_locktimeout() {
        assert_eq!(
            parse_s("SET lock_timeout = 1000"),
            StatementSummary::LockTimeout
        );
        assert_eq!(
            parse_s("SET LOCAL lock_timeout = '2s'"),
            StatementSummary::LockTimeout
        );
    }

    #[test]
    fn test_create_table() {
        assert_eq!(
            parse_s("CREATE TABLE foo (id INT)"),
            StatementSummary::CreateTable {
                schema: "".to_string(),
                name: "foo".to_string(),
                columns: vec![super::ColDefSummary {
                    name: "id".to_string(),
                    type_name: "pg_catalog.int4".to_string()
                }]
            }
        );
        assert_eq!(
            parse_s("CREATE TABLE IF NOT EXISTS public.foo (id INT)"),
            StatementSummary::CreateTable {
                schema: "public".to_string(),
                name: "foo".to_string(),
                columns: vec![super::ColDefSummary {
                    name: "id".to_string(),
                    type_name: "pg_catalog.int4".to_string()
                }]
            }
        );
        assert_eq!(
            parse_s("CREATE TABLE foo.bar (id INT)"),
            StatementSummary::CreateTable {
                schema: "foo".to_string(),
                name: "bar".to_string(),
                columns: vec![super::ColDefSummary {
                    name: "id".to_string(),
                    type_name: "pg_catalog.int4".to_string()
                }]
            }
        );
    }

    #[test]
    fn test_create_table_as() {
        assert_eq!(
            parse_s("CREATE TABLE foo AS SELECT * FROM bar"),
            StatementSummary::CreateTableAs {
                schema: "".to_string(),
                name: "foo".to_string()
            }
        );
        assert_eq!(
            parse_s("CREATE TABLE IF NOT EXISTS public.foo AS SELECT * FROM bar"),
            StatementSummary::CreateTableAs {
                schema: "public".to_string(),
                name: "foo".to_string()
            }
        );
        assert_eq!(
            parse_s("CREATE TABLE foo.bar AS SELECT * FROM bar"),
            StatementSummary::CreateTableAs {
                schema: "foo".to_string(),
                name: "bar".to_string()
            }
        );
    }

    #[test]
    fn test_create_index() {
        assert_eq!(
            parse_s("CREATE INDEX idx ON foo (bar)"),
            StatementSummary::CreateIndex {
                schema: "".to_string(),
                idxname: "idx".to_string(),
                concurrently: false,
                target: "foo".to_string()
            }
        );
        assert_eq!(
            parse_s("CREATE INDEX CONCURRENTLY idx ON foo (bar)"),
            StatementSummary::CreateIndex {
                schema: "".to_string(),
                idxname: "idx".to_string(),
                concurrently: true,
                target: "foo".to_string()
            }
        );
        assert_eq!(
            parse_s("CREATE INDEX idx ON foo.bar (baz)"),
            StatementSummary::CreateIndex {
                schema: "foo".to_string(),
                idxname: "idx".to_string(),
                concurrently: false,
                target: "bar".to_string()
            }
        );
    }

    #[test]
    fn test_set_not_null() {
        assert_eq!(
            parse_s("ALTER TABLE foo ALTER COLUMN bar SET NOT NULL"),
            StatementSummary::AlterTable {
                schema: "".to_string(),
                name: "foo".to_string(),
                actions: vec![super::AlterTableAction::SetNotNull {
                    column: "bar".to_string()
                }]
            }
        );
        assert_eq!(
            parse_s("ALTER TABLE foo.bar ALTER COLUMN baz SET NOT NULL"),
            StatementSummary::AlterTable {
                schema: "foo".to_string(),
                name: "bar".to_string(),
                actions: vec![super::AlterTableAction::SetNotNull {
                    column: "baz".to_string()
                }]
            }
        );
    }

    #[test]
    fn test_adding_not_valid_fkey() {
        assert_eq!(
            parse_s("ALTER TABLE foo ADD CONSTRAINT fkey FOREIGN KEY (bar) REFERENCES baz (id) NOT VALID"),
            StatementSummary::AlterTable {
                schema: "".to_string(),
                name: "foo".to_string(),
                actions: vec![super::AlterTableAction::AddConstraint {
                    name: "fkey".to_string(),
                    use_index: false,
                    constraint_type: pg_query::protobuf::ConstrType::ConstrForeign,
                    valid: false
                }]
            }
        );
    }

    #[test]
    fn test_adding_unique_using_index() {
        assert_eq!(
            parse_s("ALTER TABLE foo ADD CONSTRAINT unique_fkey UNIQUE USING INDEX idx"),
            StatementSummary::AlterTable {
                schema: "".to_string(),
                name: "foo".to_string(),
                actions: vec![super::AlterTableAction::AddConstraint {
                    name: "unique_fkey".to_string(),
                    use_index: true,
                    constraint_type: pg_query::protobuf::ConstrType::ConstrUnique,
                    valid: true
                }]
            }
        );
    }

    #[test]
    fn test_adding_check_not_valid() {
        assert_eq!(
            parse_s("ALTER TABLE foo ADD CONSTRAINT check_fkey CHECK (bar > 0) NOT VALID"),
            StatementSummary::AlterTable {
                schema: "".to_string(),
                name: "foo".to_string(),
                actions: vec![super::AlterTableAction::AddConstraint {
                    name: "check_fkey".to_string(),
                    use_index: false,
                    constraint_type: pg_query::protobuf::ConstrType::ConstrCheck,
                    valid: false
                }]
            }
        );
    }

    #[test]
    fn test_set_type_to_json() {
        assert_eq!(
            parse_s("ALTER TABLE foo ALTER COLUMN bar SET DATA TYPE json"),
            StatementSummary::AlterTable {
                schema: "".to_string(),
                name: "foo".to_string(),
                actions: vec![super::AlterTableAction::SetType {
                    column: "bar".to_string(),
                    type_name: "json".to_string()
                }]
            }
        );
    }

    #[test]
    fn test_add_json_column() {
        assert_eq!(
            parse_s("ALTER TABLE foo ADD COLUMN bar json"),
            StatementSummary::AlterTable {
                schema: "".to_string(),
                name: "foo".to_string(),
                actions: vec![super::AlterTableAction::AddColumn {
                    column: "bar".to_string(),
                    type_name: "json".to_string()
                }]
            }
        );
    }

    #[test]
    fn test_create_table_with_json_column() {
        assert_eq!(
            parse_s("CREATE TABLE foo (bar json)"),
            StatementSummary::CreateTable {
                schema: "".to_string(),
                name: "foo".to_string(),
                columns: vec![super::ColDefSummary {
                    name: "bar".to_string(),
                    type_name: "json".to_string()
                }]
            }
        );
    }
}
