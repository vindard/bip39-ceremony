fn main() {
    if bip39_ceremony_tui::ui::run().is_err() {
        eprintln!("bip39-ceremony could not use this terminal");
        std::process::exit(1);
    }
}
