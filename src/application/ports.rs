/// Port used by application-only live hash previews.
pub trait Sha256 {
    fn hash(&self, chunks: &[&[u8]]) -> [u8; 32];
}
