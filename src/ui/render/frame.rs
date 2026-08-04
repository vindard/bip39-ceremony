use zeroize::Zeroizing;

use crate::domain::ceremony::Phase;

use super::{App, Lines, clip, push_output};

pub(super) fn render_card(
    output: &mut String,
    title: &str,
    body: &Lines,
    rows: usize,
    requested_scroll: usize,
    focused: bool,
    width: usize,
) {
    let frame = if focused {
        CardFrame::Focused
    } else {
        CardFrame::Normal
    };
    render_card_with_frame(output, title, body, rows, requested_scroll, frame, width);
}

fn render_card_with_frame(
    output: &mut String,
    title: &str,
    body: &Lines,
    rows: usize,
    requested_scroll: usize,
    frame: CardFrame,
    width: usize,
) {
    push_output(output, &box_top(width, title, frame));
    let max_start = body.len().saturating_sub(rows.saturating_sub(1));
    let start = requested_scroll.min(max_start);
    let rail = ScrollRail::new(body.len(), rows, start, max_start);
    let mut row = 0;
    let has_above = start > 0;
    let mut visible = rows.saturating_sub(usize::from(has_above));
    let has_below = start + visible < body.len();
    visible = visible.saturating_sub(usize::from(has_below));
    let end = (start + visible).min(body.len());
    if has_above {
        push_card_row(
            output,
            "↑ more — use Page Up",
            width,
            frame,
            &rail,
            &mut row,
        );
    }
    for value in &body[start..end] {
        push_card_row(output, value, width, frame, &rail, &mut row);
    }
    if has_below {
        push_card_row(
            output,
            "↓ more — use Page Down",
            width,
            frame,
            &rail,
            &mut row,
        );
    }
    while row < rows {
        push_card_row(output, "", width, frame, &rail, &mut row);
    }
    push_output(output, &box_bottom(width, frame));
}

struct ScrollRail {
    thumb_start: usize,
    thumb_end: usize,
    enabled: bool,
}

impl ScrollRail {
    fn new(body_rows: usize, viewport_rows: usize, start: usize, max_start: usize) -> Self {
        if body_rows <= viewport_rows || viewport_rows == 0 {
            return Self {
                thumb_start: 0,
                thumb_end: 0,
                enabled: false,
            };
        }
        let thumb_len = viewport_rows
            .saturating_mul(viewport_rows)
            .checked_div(body_rows)
            .unwrap_or(1)
            .clamp(1, viewport_rows);
        let travel = viewport_rows.saturating_sub(thumb_len);
        let thumb_start = start
            .saturating_mul(travel)
            .checked_div(max_start)
            .unwrap_or(0);
        Self {
            thumb_start,
            thumb_end: thumb_start + thumb_len,
            enabled: true,
        }
    }

    const fn marker(&self, row: usize, default: char) -> char {
        if !self.enabled {
            default
        } else if row >= self.thumb_start && row < self.thumb_end {
            '█'
        } else {
            '░'
        }
    }
}

fn push_card_row(
    output: &mut String,
    value: &str,
    width: usize,
    frame: CardFrame,
    rail: &ScrollRail,
    row: &mut usize,
) {
    let line = box_row(value, width, frame, rail.marker(*row, frame.right()));
    push_output(output, &line);
    *row += 1;
}

#[derive(Clone, Copy)]
enum CardFrame {
    Normal,
    Focused,
}

impl CardFrame {
    const fn left(self) -> char {
        match self {
            Self::Normal => '│',
            Self::Focused => '┃',
        }
    }

    const fn right(self) -> char {
        self.left()
    }
}

fn box_top(width: usize, title: &str, frame: CardFrame) -> String {
    let (left, horizontal, right) = match frame {
        CardFrame::Normal => ('┌', '─', '┐'),
        CardFrame::Focused => ('┏', '━', '┓'),
    };
    let prefix = format!("{left}{horizontal} {title} ");
    let fill = width.saturating_sub(prefix.chars().count() + 1);
    format!("{prefix}{}{right}", horizontal.to_string().repeat(fill))
}

fn box_bottom(width: usize, frame: CardFrame) -> String {
    let (left, horizontal, right) = match frame {
        CardFrame::Normal => ('└', '─', '┘'),
        CardFrame::Focused => ('┗', '━', '┛'),
    };
    format!(
        "{left}{}{right}",
        horizontal.to_string().repeat(width.saturating_sub(2))
    )
}

fn box_row(value: &str, width: usize, frame: CardFrame, right_border: char) -> Zeroizing<String> {
    let inner_width = width.saturating_sub(4);
    let value = Zeroizing::new(clip(value, inner_width));
    let padding = inner_width.saturating_sub(value.chars().count());
    let left = frame.left();
    Zeroizing::new(format!(
        "{left} {}{} {right_border}",
        value.as_str(),
        " ".repeat(padding)
    ))
}

pub(super) fn title_line(width: usize, label: &str) -> String {
    let title = "BIP-39 CEREMONY";
    let gap = width.saturating_sub(title.len() + label.len());
    format!("{title}{}{label}", " ".repeat(gap.max(1)))
}

pub(super) fn stage_line(app: &App, width: usize) -> String {
    let phase = app
        .inspected_snapshot()
        .map_or(app.ceremony().state().phase(), |snapshot| snapshot.phase());
    if phase == Phase::Cancelled {
        return "× CEREMONY CANCELLED".to_owned();
    }
    let active = active_stage(phase);
    let reveal = if phase == Phase::Revealed {
        "REVEALED"
    } else {
        "REVEAL"
    };
    let full = stage_path(
        active,
        &["SETUP", "SAFETY", "ROLLS", "GENERATE", reveal],
        "  ›  ",
    );
    if full.chars().count() <= width {
        return full;
    }
    stage_path(active, &["SETUP", "SAFETY", "ROLLS", "GEN", reveal], " › ")
}

fn stage_path(active: usize, labels: &[&str; 5], separator: &str) -> String {
    labels
        .iter()
        .enumerate()
        .map(|(index, value)| {
            let marker = match index.cmp(&active) {
                std::cmp::Ordering::Less => '✓',
                std::cmp::Ordering::Equal => '●',
                std::cmp::Ordering::Greater => '○',
            };
            format!("{marker} {value}")
        })
        .collect::<Vec<_>>()
        .join(separator)
}

const fn active_stage(phase: Phase) -> usize {
    match phase {
        Phase::ChooseTarget | Phase::ChooseProtocol => 0,
        Phase::Safety => 1,
        Phase::EnterRolls => 2,
        Phase::ReadyToGenerate | Phase::ExactAttemptRejected | Phase::Result => 3,
        Phase::Revealed => 4,
        Phase::Cancelled => 5,
    }
}
