use anyhow::anyhow;

#[derive(Eq, PartialEq, Debug, Copy, Clone)]
pub enum Contype {
    Check,
    ForeignKey,
    PrimaryKey,
    Unique,
    Exclusion,
    ConstraintTrigger,
}

impl Contype {
    pub fn from_char(c: char) -> anyhow::Result<Self> {
        match c {
            'c' => Ok(Contype::Check),
            'f' => Ok(Contype::ForeignKey),
            'p' => Ok(Contype::PrimaryKey),
            'u' => Ok(Contype::Unique),
            'x' => Ok(Contype::Exclusion),
            't' => Ok(Contype::ConstraintTrigger),
            _ => Err(anyhow!("Invalid constraint type: {}", c)),
        }
    }
    pub fn to_display(&self) -> &'static str {
        match self {
            Contype::Check => "CHECK",
            Contype::ForeignKey => "FOREIGN KEY",
            Contype::PrimaryKey => "PRIMARY KEY",
            Contype::Unique => "UNIQUE",
            Contype::Exclusion => "EXCLUSION",
            Contype::ConstraintTrigger => "CONSTRAINT TRIGGER",
        }
    }
}
