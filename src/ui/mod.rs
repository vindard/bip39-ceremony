mod app;
mod render;
mod terminal;
mod theme;

pub use terminal::run;

#[cfg(test)]
impl Default for app::App {
    fn default() -> Self {
        Self::new(crate::application::CeremonySession::new(Box::new(
            crate::adapters::BitcoinSha256,
        )))
    }
}
