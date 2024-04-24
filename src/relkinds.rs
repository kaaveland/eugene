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

impl RelKind {
    pub fn from_db_str(s: char) -> Option<Self> {
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
    pub fn as_str(&self) -> &str {
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
}
