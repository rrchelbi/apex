#[derive(PartialEq, Eq, Debug, Clone, Hash, Copy)]
pub enum QueryType {
    Unknown(u16),
    A,
}

impl From<QueryType> for u16 {
    fn from(qt: QueryType) -> Self {
        match qt {
            QueryType::Unknown(x) => x,
            QueryType::A => 1,
        }
    }
}

impl From<u16> for QueryType {
    fn from(num: u16) -> Self {
        match num {
            1 => Self::A,
            _ => Self::Unknown(num),
        }
    }
}
