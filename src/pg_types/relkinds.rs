use std::fmt::Display;

/// The kind of relation, as stored in the `pg_class.relkind` column.
#[derive(Eq, PartialEq, Debug, Copy, Clone, Hash)]
pub enum RelKind {
    Table,
    Index,
    Sequence,
    Toast,
    View,
    MaterializedView,
    CompositeType,
    ForeignTable,
    PartitionedTable,
    PartitionedIndex,
}

impl Display for RelKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl RelKind {
    /// Convert a `pg_class.relkind` character code to a `RelKind`.
    pub fn from_db_code(s: char) -> Option<Self> {
        match s {
            'r' => Some(Self::Table),
            'i' => Some(Self::Index),
            'S' => Some(Self::Sequence),
            't' => Some(Self::Toast),
            'v' => Some(Self::View),
            'm' => Some(Self::MaterializedView),
            'c' => Some(Self::CompositeType),
            'f' => Some(Self::ForeignTable),
            'p' => Some(Self::PartitionedTable),
            'I' => Some(Self::PartitionedIndex),
            _ => None,
        }
    }
    /// A human readable string name for the relation kind.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Table => "Table",
            Self::Index => "Index",
            Self::Sequence => "Sequence",
            Self::Toast => "Toast",
            Self::View => "View",
            Self::MaterializedView => "MaterializedView",
            Self::CompositeType => "CompositeType",
            Self::ForeignTable => "ForeignTable",
            Self::PartitionedTable => "PartitionedTable",
            Self::PartitionedIndex => "PartitionedIndex",
        }
    }
    /// Is this RelKind an index?
    pub fn is_index(&self) -> bool {
        matches!(self, Self::Index | Self::PartitionedIndex)
    }
}
