use std::{
    io::{self, Write, stdin, stdout},
    panic,
    sync::{Arc, mpsc},
    thread,
    time::Duration,
};

use termion::{clear, cursor, input::TermRead, raw::IntoRawMode, screen::IntoAlternateScreen};

use crate::{adapters::BitcoinSha256, application::CeremonySession};

use super::{
    app::{App, UpdateOutcome},
    render::{MIN_HEIGHT, MIN_WIDTH, render, scroll_limit},
    theme::Theme,
};

/// Runs the interactive terminal ceremony.
///
/// # Errors
///
/// Returns an I/O error when the controlling terminal cannot enter raw mode,
/// receive input, or render output.
pub fn run() -> io::Result<()> {
    let previous_hook: Arc<dyn Fn(&panic::PanicHookInfo<'_>) + Send + Sync> =
        Arc::from(panic::take_hook());
    let panic_output = Arc::clone(&previous_hook);
    panic::set_hook(Box::new(move |info| {
        emergency_restore();
        panic_output(info);
    }));

    let result = run_terminal();

    let _ = panic::take_hook();
    panic::set_hook(Box::new(move |info| previous_hook(info)));
    result
}

fn run_terminal() -> io::Result<()> {
    let mut screen = stdout().into_raw_mode()?.into_alternate_screen()?;
    write!(screen, "{}{}", cursor::Hide, clear::All)?;

    let (key_sender, key_receiver) = mpsc::channel();
    thread::spawn(move || {
        for key in stdin().keys() {
            let failed = key.is_err();
            if key_sender.send(key).is_err() || failed {
                break;
            }
        }
    });

    let mut app = App::new(CeremonySession::new(Box::new(BitcoinSha256)));
    let theme = Theme::from_environment();
    let mut size = terminal_size();
    render_frame(&mut screen, &app, size, theme)?;
    loop {
        match key_receiver.recv_timeout(Duration::from_millis(100)) {
            Ok(key) => {
                let key = key?;
                if !input_allowed(size, key) {
                    continue;
                }
                match app.update_bounded(key, scroll_limit(&app, size.0, size.1)) {
                    UpdateOutcome::Unchanged => {}
                    UpdateOutcome::Changed => {
                        size = terminal_size();
                        render_frame(&mut screen, &app, size, theme)?;
                    }
                    UpdateOutcome::Exit => break,
                }
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                let next_size = terminal_size();
                if next_size != size {
                    size = next_size;
                    render_frame(&mut screen, &app, size, theme)?;
                }
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }

    write!(
        screen,
        "{}{}{}",
        clear::All,
        cursor::Goto(1, 1),
        cursor::Show
    )?;
    screen.flush()
}

fn input_allowed(size: (u16, u16), key: termion::event::Key) -> bool {
    (size.0 >= MIN_WIDTH && size.1 >= MIN_HEIGHT)
        || matches!(
            key,
            termion::event::Key::Char('q') | termion::event::Key::Ctrl('c')
        )
}

fn terminal_size() -> (u16, u16) {
    termion::terminal_size().unwrap_or((80, 24))
}

fn render_frame(
    output: &mut impl Write,
    app: &App,
    (width, height): (u16, u16),
    theme: Theme,
) -> io::Result<()> {
    let content = render(app, width, height);
    write!(output, "{}{}", clear::All, cursor::Goto(1, 1))?;
    theme.write(output, content.as_str())?;
    write!(output, "{}", cursor::Goto(1, height))?;
    output.flush()
}

fn emergency_restore() {
    let _ = write!(io::stderr(), "\x1b[0m\x1b[?25h\x1b[?1049l\r\n");
}

#[cfg(test)]
mod tests {
    use super::*;
    use termion::event::Key;

    #[test]
    fn undersized_terminal_blocks_secret_input_but_allows_exit() {
        assert!(!input_allowed((MIN_WIDTH - 1, MIN_HEIGHT), Key::Char('1')));
        assert!(!input_allowed((MIN_WIDTH, MIN_HEIGHT - 1), Key::Backspace));
        assert!(input_allowed((MIN_WIDTH - 1, MIN_HEIGHT), Key::Char('q')));
        assert!(input_allowed((MIN_WIDTH - 1, MIN_HEIGHT), Key::Ctrl('c')));
        assert!(input_allowed((MIN_WIDTH, MIN_HEIGHT), Key::Char('1')));
    }
}
