#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum TypeInformation {
    Number,
    Boolean,

    // Bool indicates wether it is owned or not
    String(bool),
}

impl TypeInformation {
    pub fn mark_owned(self) -> Self {
        match self {
            Self::String(false) => Self::String(true),
            _ => self,
        }
    }

    pub fn mark_borrowed(self) -> Self {
        match self {
            Self::String(true) => Self::String(false),
            _ => self,
        }
    }
}
