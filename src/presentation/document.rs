#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ContentBlock {
    Heading(String),
    Paragraph(String),
    Verbatim(String),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Document {
    title: String,
    blocks: Vec<ContentBlock>,
}

impl Document {
    #[must_use]
    pub const fn new(title: String, blocks: Vec<ContentBlock>) -> Self {
        Self { title, blocks }
    }

    #[must_use]
    pub fn title(&self) -> &str {
        &self.title
    }

    #[must_use]
    pub fn blocks(&self) -> &[ContentBlock] {
        &self.blocks
    }
}
