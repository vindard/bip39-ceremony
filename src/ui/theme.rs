use std::{
    env,
    io::{self, Write},
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Theme {
    Ember,
    Plain,
}

#[derive(Clone, Copy)]
struct Palette {
    primary: u8,
    accent: u8,
    muted: u8,
    success: u8,
    warning: u8,
    secret: u8,
    progress: u8,
}

#[derive(Clone, Copy, Eq, PartialEq)]
struct Style {
    color: u8,
    bold: bool,
}

impl Theme {
    pub fn from_environment() -> Self {
        let requested = env::var("BIP39_CEREMONY_THEME").ok();
        let no_color = env::var("NO_COLOR").ok();
        let term = env::var("TERM").ok();
        select(requested.as_deref(), no_color.as_deref(), term.as_deref())
    }

    pub fn write(self, output: &mut impl Write, content: &str) -> io::Result<()> {
        let Some(palette) = self.palette() else {
            return write_plain(output, content);
        };
        let lines: Vec<&str> = content.lines().collect();
        let last = lines.len().saturating_sub(1);
        let mut prev_is_card_top = false;
        let mut in_secret_card = false;
        let mut in_mnemonic = false;
        for (index, line) in lines.iter().enumerate() {
            if index > 0 {
                write!(output, "\r\n")?;
            }
            if line.starts_with('╔') {
                in_secret_card = true;
                in_mnemonic = line.contains("BIP-39 RECOVERY WORDS");
            }
            let role = LineContext {
                first: index == 0,
                last: index == last,
                card_heading: prev_is_card_top,
            };
            if in_mnemonic && line.starts_with('║') {
                write_mnemonic_line(output, line, palette)?;
            } else if in_secret_card && line.starts_with('║') {
                write_secret_card_line(output, line, palette)?;
            } else if line.contains("● REVEALED") {
                write_revealed_stage(output, line, palette)?;
            } else if line.contains('▶') {
                write_selected_line(output, line, palette)?;
            } else if line.starts_with(['│', '┃']) {
                write_card_line(output, line, role, palette)?;
            } else {
                let style = classify(line, role, palette);
                write_styled_line(output, line, style, palette.accent)?;
            }
            write!(output, "\x1b[0m")?;
            if line.starts_with('╚') {
                in_secret_card = false;
                in_mnemonic = false;
            }
            prev_is_card_top = line.starts_with(['┌', '┏']);
        }
        Ok(())
    }

    const fn palette(self) -> Option<Palette> {
        match self {
            Self::Ember => Some(Palette {
                primary: 220,
                accent: 214,
                muted: 245,
                success: 150,
                warning: 209,
                secret: 203,
                progress: 214,
            }),
            Self::Plain => None,
        }
    }
}

fn select(requested: Option<&str>, no_color: Option<&str>, term: Option<&str>) -> Theme {
    if no_color.is_some_and(|value| !value.is_empty()) || term == Some("dumb") {
        return Theme::Plain;
    }
    match requested {
        Some("plain") => Theme::Plain,
        _ => Theme::Ember,
    }
}

/// Structural facts about a line, so color follows meaning rather than the
/// line's absolute position in the composed screen.
#[derive(Clone, Copy)]
struct LineContext {
    /// The brand title on the very first row.
    first: bool,
    /// The contextual footer on the very last row.
    last: bool,
    /// The first row inside a card (the row after its top border), i.e. a heading.
    card_heading: bool,
}

fn classify(line: &str, context: LineContext, palette: Palette) -> Option<Style> {
    if context.first {
        return Some(Style {
            color: palette.primary,
            bold: true,
        });
    }
    if line.starts_with('╔') || line.starts_with('╚') {
        return Some(Style {
            color: palette.secret,
            bold: true,
        });
    }
    if line.starts_with(['┏', '┗']) {
        return Some(Style {
            color: palette.accent,
            bold: true,
        });
    }
    if line.chars().all(|character| character == '─')
        || line.starts_with('┌')
        || line.starts_with('└')
    {
        return Some(Style {
            color: palette.muted,
            bold: false,
        });
    }
    if line.contains('›') || line.starts_with('×') {
        return Some(Style {
            color: palette.muted,
            bold: false,
        });
    }
    if context.last {
        return Some(Style {
            color: palette.muted,
            bold: false,
        });
    }
    if line.starts_with('!') || line.starts_with("CANCEL") || line.contains("rejected") {
        return Some(Style {
            color: palette.warning,
            bold: true,
        });
    }
    if line.starts_with("STATE") {
        return Some(Style {
            color: palette.accent,
            bold: line.contains("FOCUS"),
        });
    }
    if line.contains("SECRET") {
        return Some(Style {
            color: palette.secret,
            bold: true,
        });
    }
    if context.card_heading {
        return Some(Style {
            color: palette.primary,
            bold: true,
        });
    }
    if line.starts_with("○ ") {
        return Some(Style {
            color: palette.primary,
            bold: false,
        });
    }
    if line.starts_with('✓') {
        return Some(Style {
            color: palette.success,
            bold: false,
        });
    }
    if line.starts_with('>') {
        return Some(Style {
            color: palette.primary,
            bold: true,
        });
    }
    if line.starts_with("encoding · ") {
        return Some(Style {
            color: palette.primary,
            bold: true,
        });
    }
    if line.starts_with('[') && line.contains('█') {
        return Some(Style {
            color: palette.progress,
            bold: false,
        });
    }
    if is_heading(line) {
        return Some(Style {
            color: palette.primary,
            bold: true,
        });
    }
    None
}

fn is_heading(line: &str) -> bool {
    let mut has_letter = false;
    for character in line.chars() {
        if character.is_lowercase() {
            return false;
        }
        has_letter |= character.is_uppercase();
    }
    has_letter
}

fn write_revealed_stage(output: &mut impl Write, line: &str, palette: Palette) -> io::Result<()> {
    let Some(active) = line.find("● REVEALED") else {
        return write!(output, "{line}");
    };
    write_style(
        output,
        Style {
            color: palette.muted,
            bold: false,
        },
    )?;
    write!(output, "{}", &line[..active])?;
    write_style(
        output,
        Style {
            color: palette.secret,
            bold: true,
        },
    )?;
    write!(output, "{}", &line[active..])
}

fn write_mnemonic_line(output: &mut impl Write, line: &str, palette: Palette) -> io::Result<()> {
    let Some(inner) = line
        .strip_prefix('║')
        .and_then(|value| value.strip_suffix('║'))
    else {
        return write!(output, "{line}");
    };
    write_secret_border(output, palette)?;
    write!(output, "║")?;

    let word = Style {
        color: palette.primary,
        bold: true,
    };
    let structure = Style {
        color: palette.muted,
        bold: false,
    };
    let mut active = None;
    for character in inner.chars() {
        let style = if character.is_ascii_lowercase() {
            word
        } else {
            structure
        };
        if active != Some(style) {
            write_style(output, style)?;
            active = Some(style);
        }
        write!(output, "{character}")?;
    }
    write_secret_border(output, palette)?;
    write!(output, "║")
}

fn write_secret_card_line(output: &mut impl Write, line: &str, palette: Palette) -> io::Result<()> {
    let Some(inner) = line
        .strip_prefix('║')
        .and_then(|value| value.strip_suffix('║'))
    else {
        return write!(output, "{line}");
    };
    write_secret_border(output, palette)?;
    write!(output, "║\x1b[0m{inner}")?;
    write_secret_border(output, palette)?;
    write!(output, "║")
}

fn write_secret_border(output: &mut impl Write, palette: Palette) -> io::Result<()> {
    write_style(
        output,
        Style {
            color: palette.secret,
            bold: true,
        },
    )
}

fn write_card_line(
    output: &mut impl Write,
    line: &str,
    role: LineContext,
    palette: Palette,
) -> io::Result<()> {
    let Some((inner, left_border, right_border)) = split_card_row(line) else {
        return write!(output, "{line}");
    };
    let focused = left_border == '┃';
    write_card_border(output, left_border, focused, palette)?;
    write!(output, "{left_border}\x1b[0m")?;
    if !write_roll_ledger_line(output, inner, palette)? {
        write_styled_line(
            output,
            inner,
            classify(inner.trim_start(), role, palette),
            palette.accent,
        )?;
    }
    write_card_border(output, right_border, focused, palette)?;
    write!(output, "{right_border}")
}

fn write_roll_ledger_line(
    output: &mut impl Write,
    inner: &str,
    palette: Palette,
) -> io::Result<bool> {
    let trimmed = inner.trim_start();
    let (latest, entry) = trimmed
        .strip_prefix("> ")
        .map_or((false, trimmed), |entry| (true, entry));
    let Some((range, rolls)) = entry.split_once("  ") else {
        return Ok(false);
    };
    let range_value = range.strip_prefix('#').unwrap_or(range);
    let range_chars = range_value.chars().collect::<Vec<_>>();
    if range_chars.len() != 7
        || range_chars[3] != '–'
        || !range_chars[..3].iter().all(char::is_ascii_digit)
        || !range_chars[4..].iter().all(char::is_ascii_digit)
    {
        return Ok(false);
    }

    write!(output, "{}", &inner[..inner.len() - trimmed.len()])?;
    if latest {
        write_style(
            output,
            Style {
                color: palette.primary,
                bold: true,
            },
        )?;
        write!(output, ">\x1b[0m ")?;
    }
    write_style(
        output,
        Style {
            color: palette.muted,
            bold: false,
        },
    )?;
    write!(output, "{range}\x1b[0m  ")?;
    let content = rolls.trim_end();
    let trailing = &rolls[content.len()..];
    if let Some(content) = content.strip_suffix('▌') {
        write!(output, "{content}")?;
        write_style(
            output,
            Style {
                color: palette.primary,
                bold: true,
            },
        )?;
        write!(output, "▌\x1b[0m{trailing}")?;
    } else {
        write!(output, "{rolls}")?;
    }
    Ok(true)
}

fn write_selected_line(output: &mut impl Write, line: &str, palette: Palette) -> io::Result<()> {
    let Some((inner, left_border, right_border)) = split_card_row(line) else {
        return write!(output, "\x1b[1;38;5;16;48;5;{}m{line}", palette.accent);
    };
    let Some(selected) = inner
        .strip_prefix(' ')
        .and_then(|value| value.strip_suffix(' '))
    else {
        return write!(output, "\x1b[1;38;5;16;48;5;{}m{line}", palette.accent);
    };
    let focused = left_border == '┃';
    write_card_border(output, left_border, focused, palette)?;
    write!(
        output,
        "{left_border} \x1b[1;38;5;16;48;5;{}m{selected}\x1b[0m",
        palette.accent
    )?;
    write_card_border(output, right_border, focused, palette)?;
    write!(output, " {right_border}")
}

fn split_card_row(line: &str) -> Option<(&str, char, char)> {
    let left_border = line.chars().next()?;
    if !matches!(left_border, '│' | '┃') {
        return None;
    }
    let inner = &line[left_border.len_utf8()..];
    let (right_index, right_border) = inner.char_indices().next_back()?;
    matches!(right_border, '│' | '┃' | '█' | '░').then_some((
        &inner[..right_index],
        left_border,
        right_border,
    ))
}

fn write_card_border(
    output: &mut impl Write,
    border: char,
    focused: bool,
    palette: Palette,
) -> io::Result<()> {
    let color = if focused || border == '█' {
        palette.accent
    } else {
        palette.muted
    };
    write_style(
        output,
        Style {
            color,
            bold: focused,
        },
    )
}

fn write_styled_line(
    output: &mut impl Write,
    line: &str,
    base: Option<Style>,
    key_color: u8,
) -> io::Result<()> {
    if let Some(style) = base {
        write_style(output, style)?;
    }
    if line.contains('█') {
        return write!(output, "{line}");
    }
    let mut remainder = line;
    while let Some(open) = remainder.find('[') {
        let Some(relative_close) = remainder[open..].find(']') else {
            break;
        };
        let close = open + relative_close + 1;
        write!(output, "{}", &remainder[..open])?;
        write_style(
            output,
            Style {
                color: key_color,
                bold: true,
            },
        )?;
        write!(output, "{}", &remainder[open..close])?;
        write!(output, "\x1b[0m")?;
        if let Some(style) = base {
            write_style(output, style)?;
        }
        remainder = &remainder[close..];
    }
    write!(output, "{remainder}")
}

fn write_style(output: &mut impl Write, style: Style) -> io::Result<()> {
    let weight = if style.bold { 1 } else { 22 };
    write!(output, "\x1b[{weight};38;5;{}m", style.color)
}

fn write_plain(output: &mut impl Write, content: &str) -> io::Result<()> {
    for (index, line) in content.lines().enumerate() {
        if index > 0 {
            write!(output, "\r\n")?;
        }
        write!(output, "{line}")?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn environment_selection_honors_accessibility_overrides() {
        assert_eq!(select(None, None, Some("xterm-256color")), Theme::Ember);
        assert_eq!(select(Some("ember"), None, None), Theme::Ember);
        assert_eq!(select(Some("ember"), Some("1"), None), Theme::Plain);
        assert_eq!(select(Some("ember"), None, Some("dumb")), Theme::Plain);
    }

    #[test]
    fn plain_theme_adds_no_terminal_escapes() {
        let mut output = Vec::new();
        Theme::Plain.write(&mut output, "one\ntwo\n").unwrap();
        assert_eq!(output, b"one\r\ntwo");
    }

    #[test]
    fn colorful_themes_style_semantic_lines() {
        let content = "BIP-39 CEREMONY\n────\nSETUP › [ROLLS]\n12 words\nSECRET\n[q] cancel";
        let mut output = Vec::new();
        Theme::Ember.write(&mut output, content).unwrap();
        let rendered = String::from_utf8(output).unwrap();
        assert!(rendered.contains("\x1b["));
        assert!(rendered.contains("SECRET"));
        assert!(rendered.contains("[q]"));

        let mut selection = Vec::new();
        Theme::Ember
            .write(&mut selection, "│   ▶ 12 words        │")
            .unwrap();
        assert!(String::from_utf8(selection).unwrap().contains("48;5;214"));
    }

    #[test]
    fn canonical_input_encoding_line_is_bold() {
        let mut output = Vec::new();
        Theme::Ember
            .write(
                &mut output,
                "BIP-39 CEREMONY\n┏━ CARD ━┓\n┃ 01 · CANONICAL INPUT ┃\n┃   encoding · ascii-rolls ┃\n┗━━━━━━━━┛",
            )
            .unwrap();
        let rendered = String::from_utf8(output).unwrap();
        // The style escape nearest before the label is bold weight (1), primary (220).
        let label = rendered.find("encoding · ascii-rolls").unwrap();
        let escape = rendered[..label].rfind("\x1b[").unwrap();
        assert!(
            rendered[escape..].starts_with("\x1b[1;38;5;220m"),
            "encoding label is not bold-primary: {:?}",
            &rendered[escape..label]
        );
    }

    #[test]
    fn card_borders_do_not_inherit_row_content_styles() {
        let mut output = Vec::new();
        Theme::Ember
            .write(
                &mut output,
                "BIP-39 CEREMONY\n┌─ CARD ─┐\n│ HEADING │\n│ [q] action █\n└────────┘",
            )
            .unwrap();
        let rendered = String::from_utf8(output).unwrap();
        assert!(rendered.matches("22;38;5;245m│").count() >= 3);
        assert!(rendered.contains("22;38;5;214m█"));
        assert!(!rendered.contains("38;5;220m│"));
        assert!(!rendered.contains("38;5;214m│"));

        let mut focused = Vec::new();
        Theme::Ember
        .write(
            &mut focused,
            "BIP-39 CEREMONY\n┏━ CARD · FOCUS ━┓\n┃ HEADING ┃\n┃ ▶ selected ┃\n┗━━━━━━━━━━━━━━━━┛",
        )
        .unwrap();
        let focused = String::from_utf8(focused).unwrap();
        assert!(focused.matches("1;38;5;214m┃").count() >= 3);
        assert!(focused.contains("1;38;5;214m┗"));
        assert!(!focused.contains("38;5;245m┃"));
    }

    #[test]
    fn focused_card_content_keeps_semantic_roles() {
        let mut output = Vec::new();
        Theme::Ember
            .write(
                &mut output,
                "BIP-39 CEREMONY\n┏━ LENGTH · FOCUS ━┓\n┃ SETUP ┃\n┃ Ordinary focused text ┃\n┃ ○ unselected ┃\n┃ ▶ selected ┃\n┗━━━━━━━━━━━━━━━━━━━━━━━┛",
            )
            .unwrap();
        let rendered = String::from_utf8(output).unwrap();
        assert!(rendered.contains("1;38;5;214m┃\x1b[0m\x1b[1;38;5;220m SETUP"));
        assert!(rendered.contains("1;38;5;214m┃\x1b[0m Ordinary focused text"));
        assert!(rendered.contains("1;38;5;214m┃\x1b[0m\x1b[22;38;5;220m ○ unselected"));
    }

    #[test]
    fn length_card_body_does_not_inherit_focused_frame_color() {
        let app = crate::ui::app::App::default();
        let screen = crate::ui::render::render(&app, 80, 40);
        let mut output = Vec::new();
        Theme::Ember.write(&mut output, &screen).unwrap();
        let rendered = String::from_utf8(output).unwrap();
        assert!(rendered.contains("1;38;5;214m┃\x1b[0m Choose mnemonic length"));
    }

    #[test]
    fn roll_capture_styles_progress_and_ledger_by_role() {
        let mut output = Vec::new();
        Theme::Ember
            .write(
                &mut output,
                "BIP-39 CEREMONY\n┏━ ROLL CAPTURE · FOCUS ━┓\n┃ PHYSICAL D6 CAPTURE · ROLLS ARE SECRET ┃\n┃ ✓ [████] 100 / 100 ┃\n┃   #001–025  12345 ┃\n┃ > #026–036  61234▌ ┃\n┗━━━━━━━━━━━━━━━━━━━━━━━━━━┛",
            )
            .unwrap();
        let rendered = String::from_utf8(output).unwrap();
        assert!(rendered.contains("38;5;150m ✓ [████] 100 / 100"));
        assert!(rendered.contains("38;5;245m#001–025\x1b[0m  12345"));
        assert!(rendered.contains("38;5;220m>\x1b[0m "));
        assert!(rendered.contains("38;5;220m▌"));
    }

    #[test]
    fn revealed_stage_and_mnemonic_have_distinct_semantic_styles() {
        let mut output = Vec::new();
        Theme::Ember
            .write(
                &mut output,
                "✓ GEN › ● REVEALED\n╔═ BIP-39 RECOVERY WORDS ╗\n║ 01  abandon           ║\n╚═══════════════════════╝",
            )
            .unwrap();
        let rendered = String::from_utf8(output).unwrap();
        assert!(rendered.contains("1;38;5;203m● REVEALED"));
        assert!(rendered.contains("1;38;5;220mabandon"));
        assert!(rendered.matches("1;38;5;203m║").count() >= 2);
    }

    #[test]
    fn unsupported_protocol_message_uses_warning_color() {
        let mut output = Vec::new();
        Theme::Ember
            .write(
                &mut output,
                "BIP-39 CEREMONY\n┏━ PROTOCOL · FOCUS ━┓\n┃     ! UNSUPPORTED · available for 24 words only ┃\n┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┛",
            )
            .unwrap();
        let rendered = String::from_utf8(output).unwrap();
        assert!(rendered.contains("1;38;5;209m     ! UNSUPPORTED"));
    }

    #[test]
    fn unselected_options_are_styled_distinctly() {
        let mut output = Vec::new();
        Theme::Ember
            .write(
                &mut output,
                "BIP-39 CEREMONY\n│   ○ 24 words        │\n[q] cancel",
            )
            .unwrap();
        let rendered = String::from_utf8(output).unwrap();
        assert!(rendered.contains("38;5;220m   ○ 24 words"));
    }
}
