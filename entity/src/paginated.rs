use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct Paginated<T> {
    pub data: Vec<T>,
    pub total: u64,
}

impl<T> Paginated<T> {
    pub fn new(data: Vec<T>, total: u64) -> Self {
        Self { data, total }
    }
}
