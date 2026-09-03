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
    white: u8,
    code_text: u8,
    code_gutter: u8,
    code_comment: u8,
    code_keyword: u8,
    code_keyword_function: u8,
    code_function: u8,
    code_type: u8,
    code_string: u8,
    code_number: u8,
    code_member: u8,
    code_parameter: u8,
    code_operator: u8,
    code_bracket: u8,
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
            } else if !write_composed_card_line(output, line, role, palette)? {
                write_regular_line(output, line, role, palette)?;
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
                white: 231,
                code_text: 189,
                code_gutter: 60,
                code_comment: 61,
                code_keyword: 183,
                code_keyword_function: 219,
                code_function: 111,
                code_type: 75,
                code_string: 150,
                code_number: 209,
                code_member: 79,
                code_parameter: 221,
                code_operator: 117,
                code_bracket: 103,
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
            color: palette.white,
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

fn write_regular_line(
    output: &mut impl Write,
    line: &str,
    role: LineContext,
    palette: Palette,
) -> io::Result<()> {
    if line.contains("● REVEALED") {
        write_revealed_stage(output, line, palette)
    } else if line.contains('▶') {
        write_selected_line(output, line, palette)
    } else if line.starts_with(['│', '┃']) {
        write_card_line(output, line, role, palette)
    } else {
        write_styled_line(output, line, classify(line, role, palette), palette.accent)
    }
}

fn write_composed_card_line(
    output: &mut impl Write,
    line: &str,
    role: LineContext,
    palette: Palette,
) -> io::Result<bool> {
    let Some(ranges) = composed_card_ranges(line) else {
        return Ok(false);
    };
    let mut end = 0;
    for range in ranges {
        write!(output, "\x1b[0m{}", &line[end..range.start])?;
        let card = &line[range.clone()];
        if card.contains('▶') {
            write_selected_line(output, card, palette)?;
        } else if card.starts_with(['│', '┃']) {
            write_card_line(output, card, role, palette)?;
        } else {
            write_styled_line(output, card, classify(card, role, palette), palette.accent)?;
        }
        end = range.end;
    }
    write!(output, "\x1b[0m{}", &line[end..])?;
    Ok(true)
}

fn composed_card_ranges(line: &str) -> Option<Vec<std::ops::Range<usize>>> {
    let mut ranges = Vec::new();
    let mut start = 0;
    loop {
        let opening = line[start..].chars().next()?;
        let closing = match opening {
            '┌' => &['┐'][..],
            '┏' => &['┓'][..],
            '└' => &['┘'][..],
            '┗' => &['┛'][..],
            '│' | '┃' => &['│', '┃', '█', '░'][..],
            _ => return None,
        };
        let offset = opening.len_utf8();
        let (relative_end, character) =
            line[start + offset..]
                .char_indices()
                .find(|(index, character)| {
                    if !closing.contains(character) {
                        return false;
                    }
                    let end = start + offset + index + character.len_utf8();
                    let remainder = &line[end..];
                    remainder.is_empty()
                        || remainder.starts_with(" ┌")
                        || remainder.starts_with(" ┏")
                        || remainder.starts_with(" └")
                        || remainder.starts_with(" ┗")
                        || remainder.starts_with(" │")
                        || remainder.starts_with(" ┃")
                })?;
        let end = start + offset + relative_end + character.len_utf8();
        ranges.push(start..end);
        if end == line.len() {
            break;
        }
        start = end + 1;
    }
    (ranges.len() > 1).then_some(ranges)
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
    write_card_border(output, focused, palette)?;
    write!(output, "{left_border}\x1b[0m")?;
    if !write_rust_source_line(output, inner, palette)?
        && !write_roll_ledger_line(output, inner, palette)?
    {
        write_styled_line(
            output,
            inner,
            classify(inner.trim_start(), role, palette),
            palette.accent,
        )?;
    }
    write_card_border(output, focused, palette)?;
    write!(output, "{right_border}")
}

#[derive(Clone, Copy)]
enum CodeStyle {
    Text,
    Gutter,
    Comment,
    Keyword,
    KeywordFunction,
    Function,
    Type,
    String,
    Number,
    Member,
    Parameter,
    Operator,
    Bracket,
}

fn write_rust_source_line(
    output: &mut impl Write,
    inner: &str,
    palette: Palette,
) -> io::Result<bool> {
    let trimmed = inner.trim_start();
    let leading = &inner[..inner.len() - trimmed.len()];
    let Some((line_number, code)) = trimmed.split_once(" │ ") else {
        return Ok(false);
    };
    if line_number.len() != 4 || !line_number.bytes().all(|byte| byte.is_ascii_digit()) {
        return Ok(false);
    }

    write!(output, "{leading}")?;
    write_code_token(output, line_number, CodeStyle::Gutter, palette)?;
    write_code_token(output, " │ ", CodeStyle::Bracket, palette)?;
    write_rust_code(output, code, palette)?;
    Ok(true)
}

fn write_rust_code(output: &mut impl Write, code: &str, palette: Palette) -> io::Result<()> {
    let mut offset = 0;
    while offset < code.len() {
        let token = next_rust_token(&code[..offset], &code[offset..]);
        let end = offset + token.length;
        if let Some(style) = token.style {
            write_code_token(output, &code[offset..end], style, palette)?;
        } else {
            write!(output, "{}", &code[offset..end])?;
        }
        offset = end;
    }
    Ok(())
}

struct RustToken {
    length: usize,
    style: Option<CodeStyle>,
}

fn next_rust_token(before: &str, remaining: &str) -> RustToken {
    let character = remaining.chars().next().expect("source token is non-empty");
    match character {
        '/' if remaining.starts_with("//") => styled_token(remaining.len(), CodeStyle::Comment),
        '"' => styled_token(quoted_length(remaining, '"'), CodeStyle::String),
        '\'' => rust_apostrophe_token(remaining),
        value if value.is_ascii_digit() => {
            styled_token(number_length(remaining), CodeStyle::Number)
        }
        value if value.is_ascii_alphabetic() || value == '_' => {
            let length = identifier_length(remaining);
            styled_token(
                length,
                classify_rust_identifier(before, &remaining[..length], &remaining[length..]),
            )
        }
        value if value.is_whitespace() => RustToken {
            length: value.len_utf8(),
            style: None,
        },
        '(' | ')' | '[' | ']' | '{' | '}' => styled_token(character.len_utf8(), CodeStyle::Bracket),
        _ => styled_token(character.len_utf8(), CodeStyle::Operator),
    }
}

const fn styled_token(length: usize, style: CodeStyle) -> RustToken {
    RustToken {
        length,
        style: Some(style),
    }
}

fn rust_apostrophe_token(remaining: &str) -> RustToken {
    char_literal_length(remaining).map_or_else(
        || styled_token(1 + identifier_length(&remaining[1..]), CodeStyle::Keyword),
        |length| styled_token(length, CodeStyle::String),
    )
}

fn char_literal_length(value: &str) -> Option<usize> {
    let mut characters = value.char_indices();
    characters.next()?;
    let (_, first) = characters.next()?;
    if first == '\\' {
        characters.next()?;
    }
    let (offset, closing) = characters.next()?;
    (closing == '\'').then_some(offset + closing.len_utf8())
}

fn quoted_length(value: &str, quote: char) -> usize {
    let mut escaped = false;
    for (offset, character) in value.char_indices().skip(1) {
        if character == quote && !escaped {
            return offset + character.len_utf8();
        }
        escaped = character == '\\' && !escaped;
        if character != '\\' {
            escaped = false;
        }
    }
    value.len()
}

fn identifier_length(value: &str) -> usize {
    value
        .char_indices()
        .take_while(|(_, character)| character.is_ascii_alphanumeric() || *character == '_')
        .last()
        .map_or(0, |(offset, character)| offset + character.len_utf8())
}

fn number_length(value: &str) -> usize {
    let bytes = value.as_bytes();
    let mut length = 0;
    while let Some(&byte) = bytes.get(length) {
        let decimal_point = byte == b'.' && bytes.get(length + 1) != Some(&b'.');
        if byte.is_ascii_alphanumeric() || byte == b'_' || decimal_point {
            length += 1;
        } else {
            break;
        }
    }
    length
}

fn classify_rust_identifier(before: &str, identifier: &str, after: &str) -> CodeStyle {
    let following = after.trim_start();
    if identifier == "fn" {
        return CodeStyle::KeywordFunction;
    }
    if is_rust_keyword(identifier) {
        return CodeStyle::Keyword;
    }
    if matches!(
        identifier,
        "None" | "Some" | "Ok" | "Err" | "true" | "false"
    ) {
        return CodeStyle::Number;
    }
    if identifier.starts_with(char::is_uppercase) && before.trim_end().ends_with("::") {
        return CodeStyle::Number;
    }
    if is_rust_primitive(identifier) || identifier.starts_with(char::is_uppercase) {
        return CodeStyle::Type;
    }
    if following.starts_with('!') || following.starts_with('(') {
        return CodeStyle::Function;
    }
    let before = before.trim_end();
    if before.ends_with('.') && !before.ends_with("..") {
        return CodeStyle::Member;
    }
    if following.starts_with(':') && !following.starts_with("::") {
        return if before.rfind('(') > before.rfind(')') {
            CodeStyle::Parameter
        } else {
            CodeStyle::Member
        };
    }
    if following.starts_with("::") {
        return CodeStyle::Keyword;
    }
    CodeStyle::Text
}

fn is_rust_keyword(value: &str) -> bool {
    matches!(
        value,
        "as" | "async"
            | "await"
            | "break"
            | "const"
            | "continue"
            | "crate"
            | "dyn"
            | "else"
            | "enum"
            | "extern"
            | "for"
            | "if"
            | "impl"
            | "in"
            | "let"
            | "loop"
            | "match"
            | "mod"
            | "move"
            | "mut"
            | "pub"
            | "ref"
            | "return"
            | "self"
            | "static"
            | "struct"
            | "super"
            | "trait"
            | "type"
            | "union"
            | "unsafe"
            | "use"
            | "where"
            | "while"
            | "yield"
    )
}

fn is_rust_primitive(value: &str) -> bool {
    matches!(
        value,
        "bool"
            | "char"
            | "f32"
            | "f64"
            | "i8"
            | "i16"
            | "i32"
            | "i64"
            | "i128"
            | "isize"
            | "str"
            | "u8"
            | "u16"
            | "u32"
            | "u64"
            | "u128"
            | "usize"
    )
}

fn write_code_token(
    output: &mut impl Write,
    token: &str,
    style: CodeStyle,
    palette: Palette,
) -> io::Result<()> {
    let (color, italic) = match style {
        CodeStyle::Text => (palette.code_text, false),
        CodeStyle::Gutter => (palette.code_gutter, false),
        CodeStyle::Comment => (palette.code_comment, true),
        CodeStyle::Keyword => (palette.code_keyword, true),
        CodeStyle::KeywordFunction => (palette.code_keyword_function, false),
        CodeStyle::Function => (palette.code_function, false),
        CodeStyle::Type => (palette.code_type, false),
        CodeStyle::String => (palette.code_string, false),
        CodeStyle::Number => (palette.code_number, false),
        CodeStyle::Member => (palette.code_member, false),
        CodeStyle::Parameter => (palette.code_parameter, false),
        CodeStyle::Operator => (palette.code_operator, false),
        CodeStyle::Bracket => (palette.code_bracket, false),
    };
    let slant = if italic { 3 } else { 23 };
    write!(output, "\x1b[{slant};22;38;5;{color}m{token}\x1b[0m")
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
    write_card_border(output, focused, palette)?;
    write!(
        output,
        "{left_border} \x1b[1;38;5;16;48;5;{}m{selected}\x1b[0m",
        palette.accent
    )?;
    write_card_border(output, focused, palette)?;
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

fn write_card_border(output: &mut impl Write, focused: bool, palette: Palette) -> io::Result<()> {
    let color = if focused {
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
    fn canonical_input_encoding_line_is_bold_white() {
        let mut output = Vec::new();
        Theme::Ember
            .write(
                &mut output,
                "BIP-39 CEREMONY\n┏━ CARD ━┓\n┃ 01 · CANONICAL INPUT ┃\n┃   encoding · ascii-rolls ┃\n┗━━━━━━━━┛",
            )
            .unwrap();
        let rendered = String::from_utf8(output).unwrap();
        // The style escape nearest before the label is bold weight (1), white (231) —
        // deliberately not the yellow primary (220) used for other headings.
        let label = rendered.find("encoding · ascii-rolls").unwrap();
        let escape = rendered[..label].rfind("\x1b[").unwrap();
        assert!(
            rendered[escape..].starts_with("\x1b[1;38;5;231m"),
            "encoding label is not bold-white: {:?}",
            &rendered[escape..label]
        );
    }

    #[test]
    fn card_borders_use_one_color_per_focus_state() {
        let mut output = Vec::new();
        Theme::Ember
            .write(
                &mut output,
                "BIP-39 CEREMONY\n┌─ CARD ─┐\n│ HEADING │\n│ [q] action █\n└────────┘",
            )
            .unwrap();
        let rendered = String::from_utf8(output).unwrap();
        assert!(rendered.matches("22;38;5;245m│").count() >= 3);
        assert!(rendered.contains("22;38;5;245m█"));
        assert!(!rendered.contains("38;5;220m│"));
        assert!(!rendered.contains("38;5;214m│"));
        assert!(!rendered.contains("38;5;214m█"));

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
    fn composed_cards_keep_independent_borders_and_selection() {
        let mut output = Vec::new();
        Theme::Ember
            .write(
                &mut output,
                "BIP-39 CEREMONY\n┌─ STAGES ─┐ ┏━ TASK · FOCUS ━┓ ┌─ PREVIEW ─┐\n│ Stage │ ┃ ▶ selected ┃ │ Preview █\n└──────────┘ ┗━━━━━━━━━━━━━━━━━━┛ └─────────────┘",
            )
            .unwrap();
        let rendered = String::from_utf8(output).unwrap();
        assert_eq!(rendered.matches("48;5;214").count(), 1);
        assert!(rendered.contains("22;38;5;245m┌"));
        assert!(rendered.contains("1;38;5;214m┏"));
        assert!(rendered.contains("22;38;5;245m█"));
        assert!(rendered.contains("1;38;5;214m┃"));
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
    fn rust_source_uses_editor_style_semantic_colors() {
        let mut output = Vec::new();
        Theme::Ember
            .write(
                &mut output,
                "BIP-39 CEREMONY\n┏━ SOURCE · FOCUS ━┓\n┃ 0009 │     mnemonic: EnglishMnemonic, ┃\n┃ 0015 │ pub(crate) fn from_entropy(entropy: &Entropy) -> Result<Self, Bip39Error> { ┃\n┃ 0016 │     return Err(\"mismatch\"); // rejected ┃\n┃ 0036 │     let checksum_bits = (0..checksum_width); ┃\n┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┛",
            )
            .unwrap();
        let rendered = String::from_utf8(output).unwrap();

        for expected in [
            "23;22;38;5;60m0015",
            "3;22;38;5;183mpub",
            "23;22;38;5;219mfn",
            "23;22;38;5;111mfrom_entropy",
            "23;22;38;5;221mentropy",
            "23;22;38;5;79mmnemonic",
            "23;22;38;5;75mEntropy",
            "23;22;38;5;209mErr",
            "23;22;38;5;150m\"mismatch\"",
            "3;22;38;5;61m// rejected",
            "23;22;38;5;117m&",
            "23;22;38;5;103m(",
            "23;22;38;5;209m0",
            "23;22;38;5;117m.",
            "23;22;38;5;189mchecksum_width",
        ] {
            assert!(rendered.contains(expected), "missing style for {expected}");
        }
        assert!(rendered.contains("1;38;5;214m┃"));
    }

    #[test]
    fn rust_styling_preserves_every_source_character() {
        for source in [
            " 0002 │                                                                ",
            " 0021 │ pub(super) fn from_encoded(value: &'static str) -> Result<u16, Error> { // exact  ",
            " 0036 │     let checksum_bits = (0..checksum_width);                    ",
        ] {
            let mut output = Vec::new();
            assert!(
                write_rust_source_line(&mut output, source, Theme::Ember.palette().unwrap())
                    .unwrap()
            );
            let rendered = String::from_utf8(output).unwrap();
            assert_eq!(without_sgr(&rendered), source);
        }
    }

    #[test]
    fn rust_source_activation_requires_a_four_digit_gutter() {
        let mut output = Vec::new();

        assert!(
            !write_rust_source_line(
                &mut output,
                " label │ value ",
                Theme::Ember.palette().unwrap()
            )
            .unwrap()
        );
        assert!(output.is_empty());
    }

    fn without_sgr(value: &str) -> String {
        let mut segments = value.split('\x1b');
        let mut plain = segments.next().unwrap_or_default().to_owned();
        for segment in segments {
            let (_, text) = segment.split_once('m').expect("theme emits SGR escapes");
            plain.push_str(text);
        }
        plain
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
