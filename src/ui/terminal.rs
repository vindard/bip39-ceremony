use std::{
    io::{self, Write, stdin, stdout},
    panic,
    sync::{Arc, mpsc},
    thread,
    time::Duration,
};

use termion::{
    clear, cursor,
    event::{Event, Key, MouseButton, MouseEvent},
    input::{MouseTerminal, TermRead},
    raw::IntoRawMode,
    screen::IntoAlternateScreen,
};

use crate::{adapters::BitcoinSha256, application::CeremonySession};

use super::{
    app::{App, UpdateOutcome},
    render::{minimum_size, render, scroll_limit, workspace_pane_at},
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
    let screen = stdout().into_raw_mode()?.into_alternate_screen()?;
    let mut screen = MouseTerminal::from(screen);
    write!(screen, "{}{}", cursor::Hide, clear::All)?;

    let (key_sender, key_receiver) = mpsc::channel();
    thread::spawn(move || {
        for event in stdin().events() {
            let failed = event.is_err();
            if key_sender.send(event).is_err() || failed {
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
            Ok(event) => match event? {
                Event::Key(key) => {
                    if !input_allowed(&app, size, key) {
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
                Event::Mouse(MouseEvent::Press(MouseButton::Left, column, row)) => {
                    if let Some(pane) = workspace_pane_at(&app, size.0, size.1, column, row)
                        && app.focus_workspace_pane(pane)
                    {
                        render_frame(&mut screen, &app, size, theme)?;
                    }
                }
                Event::Mouse(_) | Event::Unsupported(_) => {}
            },
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

fn input_allowed(app: &App, size: (u16, u16), key: Key) -> bool {
    let minimum = minimum_size(app);
    (size.0 >= minimum.0 && size.1 >= minimum.1)
        || matches!(key, Key::Char('q') | Key::Ctrl('c'))
        || (app.ceremony().state().phase() == crate::domain::ceremony::Phase::Revealed
            && matches!(key, Key::Char('h')))
        || (app.group_revealed() && matches!(key, Key::Char('r')))
        || (app.group_derivation().is_some() && matches!(key, Key::Char('d') | Key::Esc))
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
    let _ = write!(
        io::stderr(),
        "\x1b[0m\x1b[?1006l\x1b[?1015l\x1b[?1002l\x1b[?1000l\x1b[?25h\x1b[?1049l\r\n"
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use termion::event::Key;

    #[test]
    fn undersized_terminal_blocks_secret_input_but_allows_exit() {
        let app = App::default();
        let minimum = minimum_size(&app);
        assert!(!input_allowed(
            &app,
            (minimum.0 - 1, minimum.1),
            Key::Char('1')
        ));
        assert!(!input_allowed(
            &app,
            (minimum.0, minimum.1 - 1),
            Key::Backspace
        ));
        assert!(input_allowed(
            &app,
            (minimum.0 - 1, minimum.1),
            Key::Char('q')
        ));
        assert!(input_allowed(
            &app,
            (minimum.0 - 1, minimum.1),
            Key::Ctrl('c')
        ));
        assert!(input_allowed(&app, (minimum.0, minimum.1), Key::Char('1')));
    }

    #[test]
    fn undersized_group_secret_views_can_be_concealed() {
        let mut app = App::default();
        app.update(Key::Char('\n'));
        app.update(Key::Char('g'));
        for index in 0..50 {
            let face = char::from(b'1' + u8::try_from(index % 6).unwrap());
            app.update(Key::Char(face));
        }
        app.update(Key::Char('\n'));
        app.update(Key::Char('r'));

        assert_eq!(minimum_size(&app), (52, 40));
        assert!(input_allowed(&app, (44, 22), Key::Char('r')));
        assert!(!input_allowed(&app, (44, 22), Key::Char('1')));

        app.update(Key::Char('r'));
        app.update(Key::Char('d'));
        assert_eq!(minimum_size(&app), (52, 40));
        assert!(input_allowed(&app, (44, 22), Key::Char('d')));
        assert!(input_allowed(&app, (44, 22), Key::Esc));
    }

    #[test]
    fn undersized_reveal_still_allows_immediate_concealment() {
        let mut app = App::default();
        app.update(Key::Char('\n'));
        app.update(Key::Down);
        app.update(Key::Down);
        app.update(Key::Char('\n'));
        app.update(Key::Char('c'));
        app.update(Key::Char('\n'));
        for _ in 0..50 {
            app.update(Key::Char('1'));
        }
        app.update(Key::Char('\n'));
        app.update(Key::Char('r'));

        let minimum = minimum_size(&app);
        assert_eq!(minimum, (52, 40));
        assert!(input_allowed(&app, (44, 22), Key::Char('h')));
        assert!(!input_allowed(&app, (44, 22), Key::Char('1')));
    }
}
