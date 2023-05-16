
pub type DocumentId = i64;

pub trait Identifiable {
    fn as_i64(&self) -> DocumentId;
} 