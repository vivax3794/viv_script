#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum TypeInformation {
    Number,

    // Both of these are stored as just a i8*
    // But the Borrow/Owned signals wether the current code needs to free the value (and wether it is free to)
    // For example a print can work on both without doing extra work
    // while a var needs to convert a borrow to owned (by cloning it)
    StringBorrow,
    StringOwned,
}
