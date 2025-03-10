use crate::error::ContextualError;
use crate::lints::ast::AstError::{ColDefMissingTypeName, MissingRelation};
use log::trace;
use pg_query::protobuf::node::Node;
use pg_query::protobuf::{
    AlterTableCmd, AlterTableType, ColumnDef, ConstrType, CreateEnumStmt, CreateStmt,
    CreateTableAsStmt, IndexStmt, VariableSetStmt,
};
use pg_query::NodeRef;

#[derive(Debug)]
pub enum AstError {
    MissingRelation,
    ColDefMissingTypeName,
    UnrecognizedAltCmdSubType(i32),
    UnrecognizedConstraintType(i32),
    ExpectedConstraintDef,
    ExpectedColDef,
    ExpectedCommandNode,
    ExpectEnumTypeName,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColDefSummary {
    pub name: String,
    pub type_name: String,
    pub stored_generated: bool,
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
    CreateEnum {
        name: String,
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
            | StatementSummary::CreateEnum { .. }
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
            StatementSummary::CreateEnum { .. } => vec![],
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
        stored_generated: bool,
    },
    Unrecognized,
}

fn set_statement(child: &VariableSetStmt) -> crate::Result<StatementSummary> {
    if child.name.eq_ignore_ascii_case("lock_timeout") {
        Ok(StatementSummary::LockTimeout)
    } else {
        Ok(StatementSummary::Ignored)
    }
}

fn create_table(child: &CreateStmt) -> crate::Result<StatementSummary> {
    trace!("create_table: {:?}", child);
    if let Some(rel) = &child.relation {
        let schema = rel.schemaname.clone();
        let name = rel.relname.clone();
        let elts: crate::Result<Vec<_>> = child
            .table_elts
            .iter()
            .map(|node| {
                let inner = node.node.as_ref().map(|node| node.to_ref());
                trace!("create_table elt: {:?}", inner);
                if let Some(NodeRef::ColumnDef(coldef)) = inner {
                    let name = coldef.colname.clone();
                    let type_name = col_type_as_string(coldef)?;
                    let stored_generated = stored_generated(coldef);
                    Ok(Some(ColDefSummary {
                        name,
                        type_name,
                        stored_generated,
                    }))
                } else {
                    Ok(None)
                }
            })
            .collect();
        Ok(StatementSummary::CreateTable {
            schema,
            name,
            columns: elts?.into_iter().flatten().collect(),
        })
    } else {
        Err(AstError::MissingRelation
            .with_context("CREATE TABLE statement does not have a relation"))
    }
}

fn stored_generated(coldef: &ColumnDef) -> bool {
    coldef.constraints.iter().any(|c| match c.node.as_ref() {
        Some(Node::Constraint(cons)) => {
            &cons.generated_when == "a"
                && ConstrType::from_i32(cons.contype) == Some(ConstrType::ConstrGenerated)
        }
        _ => false,
    })
}

fn create_table_as(child: &CreateTableAsStmt) -> crate::Result<StatementSummary> {
    let out = if let Some(dest) = &child.into {
        if let Some(rel) = &dest.rel {
            let schema = rel.schemaname.clone();
            let name = rel.relname.clone();
            Some(StatementSummary::CreateTableAs { schema, name })
        } else {
            None
        }
    } else {
        None
    };
    out.ok_or_else(|| {
        MissingRelation.with_context("CREATE TABLE AS statement does not have a relation")
    })
}

fn create_index(child: &IndexStmt) -> crate::Result<StatementSummary> {
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
        Err(MissingRelation.with_context("CREATE INDEX statement does not have a relation"))
    }
}

fn col_type_as_string(coldef: &ColumnDef) -> crate::Result<String> {
    trace!("col_type_as_string: {:?}", coldef);
    if let Some(tp) = &coldef.type_name {
        let names: crate::Result<Vec<String>> = tp
            .names
            .iter()
            .map(|n| match n.node.as_ref() {
                Some(Node::String(tn)) => Ok(tn.sval.to_owned()),
                _ => Err(ColDefMissingTypeName
                    .with_context(format!("Column definition has no type name: {n:?}"))),
            })
            .collect();
        Ok(names?.join("."))
    } else {
        Err(ColDefMissingTypeName.into())
    }
}

fn parse_alter_table_action(child: &AlterTableCmd) -> crate::Result<AlterTableAction> {
    let subtype = AlterTableType::from_i32(child.subtype)
        .ok_or(AstError::UnrecognizedAltCmdSubType(child.subtype))?;

    trace!("parse_alter_table_action: {:?} {:?}", subtype, child);
    match subtype {
        AlterTableType::AtAlterColumnType => {
            let col = expect_coldef(child)?;
            Ok(AlterTableAction::SetType {
                column: child.name.clone(),
                type_name: col_type_as_string(col)?,
            })
        }
        AlterTableType::AtAddColumn => {
            let col = expect_coldef(child)?;
            let stored_generated = stored_generated(col);
            Ok(AlterTableAction::AddColumn {
                column: col.colname.clone(),
                type_name: col_type_as_string(col)?,
                stored_generated,
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
                .ok_or(AstError::UnrecognizedConstraintType(constraint_type))?;

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

fn expect_constraint_def(child: &AlterTableCmd) -> crate::Result<&pg_query::protobuf::Constraint> {
    trace!("expect_constraint_def: {:?}", child);
    if let Some(def) = &child.def {
        let next = def.node.as_ref();
        if let Some(n) = next {
            if let NodeRef::Constraint(constraint) = n.to_ref() {
                Ok(constraint)
            } else {
                Err(AstError::ExpectedConstraintDef.with_context(format!(
                    "AlterTableCmd Expected constraint def, found: {n:?}"
                )))
            }
        } else {
            Err(AstError::ExpectedConstraintDef.into())
        }
    } else {
        Err(AstError::ExpectedConstraintDef.into())
    }
}

fn expect_coldef(child: &AlterTableCmd) -> crate::Result<&ColumnDef> {
    trace!("expect_coldef: {:?}", child);
    if let Some(def) = &child.def {
        let next = def.node.as_ref();
        if let Some(n) = next {
            if let NodeRef::ColumnDef(colddef) = n.to_ref() {
                Ok(colddef)
            } else {
                Err(AstError::ExpectedColDef
                    .with_context(format!("AlterTableCmd Expected column def, found: {n:?}")))
            }
        } else {
            Err(AstError::ExpectedColDef.into())
        }
    } else {
        Err(AstError::ExpectedColDef.into())
    }
}

fn alter_table(child: &pg_query::protobuf::AlterTableStmt) -> crate::Result<StatementSummary> {
    if let Some(rel) = &child.relation {
        let schema = rel.schemaname.clone();
        let name = rel.relname.clone();
        let actions: crate::Result<Vec<_>> = child
            .cmds
            .iter()
            .map(|cmd| {
                if let Some(cmd_node) = &cmd.node {
                    let node_ref = &cmd_node.to_ref();
                    if let NodeRef::AlterTableCmd(child) = node_ref {
                        parse_alter_table_action(child)
                    } else {
                        Err(AstError::ExpectedCommandNode.with_context(format!(
                            "ALTER TABLE statement has an unrecognized command node: {node_ref:?}"
                        )))
                    }
                } else {
                    Err(AstError::ExpectedCommandNode.into())
                }
            })
            .collect();
        Ok(StatementSummary::AlterTable {
            schema,
            name,
            actions: actions?,
        })
    } else {
        Err(MissingRelation.with_context("ALTER TABLE statement does not have a relation"))
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
pub fn describe(statement: &NodeRef) -> crate::Result<StatementSummary> {
    trace!("receiving {:?}", statement);
    match statement {
        NodeRef::VariableSetStmt(child) => set_statement(child),
        // CREATE TABLE
        NodeRef::CreateStmt(child) => create_table(child),
        // CREATE TABLE AS
        NodeRef::CreateTableAsStmt(child) => create_table_as(child),
        // CREATE INDEX
        NodeRef::IndexStmt(child) => create_index(child),
        NodeRef::AlterTableStmt(child) => alter_table(child),
        NodeRef::CreateEnumStmt(child) => create_enum(child),
        _ => Ok(StatementSummary::Ignored),
    }
}

fn create_enum(stmt: &CreateEnumStmt) -> crate::Result<StatementSummary> {
    let name_parts: crate::Result<Vec<_>> = stmt
        .type_name
        .iter()
        .map(|n| {
            if let Some(Node::String(s)) = n.node.as_ref() {
                Ok(s.sval.clone())
            } else {
                Err(AstError::ExpectEnumTypeName
                    .with_context(format!("Expected Node::String type node got {n:?}")))
            }
        })
        .collect();
    Ok(StatementSummary::CreateEnum {
        name: name_parts?.join("."),
    })
}

#[cfg(test)]
mod tests {
    use crate::lints::StatementSummary;
    use pretty_assertions::assert_eq;

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
                    type_name: "pg_catalog.int4".to_string(),
                    stored_generated: false
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
                    type_name: "pg_catalog.int4".to_string(),
                    stored_generated: false
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
                    type_name: "pg_catalog.int4".to_string(),
                    stored_generated: false
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
                name: "foo".to_string(),
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
                    type_name: "pg_catalog.json".to_string()
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
                    type_name: "pg_catalog.json".to_string(),
                    stored_generated: false
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
                    type_name: "pg_catalog.json".to_string(),
                    stored_generated: false
                }]
            }
        );
    }
}
