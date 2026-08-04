use crate::domain::{
    dice::RollSequence,
    protocol::{
        AssignmentStatus, CandidatePurpose, CandidateStatus, WordExactCandidate, WordExactTrace,
        trace_word_exact,
    },
};

use super::{App, Lines, push, push_wrapped};

pub(super) fn render_word_exact_assignment(output: &mut Lines, app: &App, width: usize) {
    let state = app.ceremony().state();
    let Some(target) = state.target() else {
        return;
    };
    let trace = trace_word_exact(target, state.rolls());
    let candidates = trace.candidates();

    render_active_candidate(output, app, candidates, target.word_count());
    render_assignment_map(output, &trace, target.word_count());
    render_candidate_history(
        output,
        candidates,
        target.word_count(),
        state.rolls(),
        app.rolls_hidden(),
        width,
    );
}

fn render_active_candidate(
    output: &mut Lines,
    app: &App,
    candidates: &[WordExactCandidate],
    final_word: usize,
) {
    let Some(active) = candidates
        .iter()
        .rev()
        .find(|candidate| candidate.status() == CandidateStatus::Collecting)
    else {
        return;
    };
    push(output, "CURRENT CANDIDATE");
    push(output, &candidate_heading(*active, final_word));
    push(
        output,
        &candidate_faces(*active, app.ceremony().state().rolls(), app.rolls_hidden()),
    );
    push(
        output,
        &format!(
            "  {} / {} recorded · acceptance known only when complete",
            active.recorded_rolls(),
            active.required_rolls()
        ),
    );
    push(output, "");
}

fn render_assignment_map(output: &mut Lines, trace: &WordExactTrace, final_word: usize) {
    push(output, "ASSIGNMENT MAP");
    let markers = (1..=trace.required_words())
        .map(|position| position_marker(trace, position))
        .collect::<Vec<_>>();
    for row in markers.chunks(6) {
        push(output, &format!("  {}", row.join(" ")));
    }
    push(
        output,
        &format!(
            "  {final_word:02} {} FINAL ENTROPY + CALCULATED CHECKSUM",
            assignment_marker(trace.assignment_status(CandidatePurpose::EntropyTail))
        ),
    );
    push(
        output,
        "  ✓ accepted · ✓+ accepted after retry · ◐ collecting · ○ waiting",
    );
}

fn position_marker(trace: &WordExactTrace, position: usize) -> String {
    let marker =
        assignment_marker(trace.assignment_status(CandidatePurpose::WordPosition(position)));
    format!("{position:02} {marker:<2}")
}

const fn assignment_marker(status: AssignmentStatus) -> &'static str {
    match status {
        AssignmentStatus::Waiting => "○",
        AssignmentStatus::Collecting => "◐",
        AssignmentStatus::Accepted { retried: false } => "✓",
        AssignmentStatus::Accepted { retried: true } => "✓+",
    }
}

fn render_candidate_history(
    output: &mut Lines,
    candidates: &[WordExactCandidate],
    final_word: usize,
    rolls: &RollSequence,
    faces_hidden: bool,
    width: usize,
) {
    let completed = candidates
        .iter()
        .filter(|candidate| candidate.status() != CandidateStatus::Collecting)
        .copied()
        .collect::<Vec<_>>();
    if completed.is_empty() {
        return;
    }
    push(output, "");
    push(output, "CANDIDATE HISTORY");
    for candidate in completed {
        let status = match candidate.status() {
            CandidateStatus::Accepted => "✓ KEPT FOR ENTROPY",
            CandidateStatus::Rejected => "× REJECTED · KEPT IN AUDIT",
            CandidateStatus::Collecting => unreachable!("completed candidates were filtered"),
        };
        let faces = if faces_hidden {
            zeroize::Zeroizing::new(String::new())
        } else {
            let candidate_faces = candidate_faces(candidate, rolls, false);
            zeroize::Zeroizing::new(format!(" · {}", candidate_faces.trim()))
        };
        let line = zeroize::Zeroizing::new(format!(
            "{} · {status}{}",
            candidate_heading(candidate, final_word).trim(),
            faces.as_str()
        ));
        push_wrapped(output, "  ", &line, width);
    }
}

fn candidate_heading(candidate: WordExactCandidate, final_word: usize) -> String {
    let purpose = match candidate.purpose() {
        CandidatePurpose::WordPosition(position) => format!("POSITION {position:02}"),
        CandidatePurpose::EntropyTail => format!("FINAL WORD {final_word:02} ENTROPY"),
    };
    let last_roll = candidate.first_roll() + candidate.required_rolls() - 1;
    format!(
        "  {purpose} · ATTEMPT {} · #{:03}–{last_roll:03}",
        candidate.attempt(),
        candidate.first_roll()
    )
}

fn candidate_faces(
    candidate: WordExactCandidate,
    rolls: &RollSequence,
    prior_rolls_hidden: bool,
) -> zeroize::Zeroizing<String> {
    let latest = rolls.len();
    let mut cells = zeroize::Zeroizing::new(String::from("  ["));
    for offset in 0..candidate.required_rolls() {
        if offset > 0 {
            cells.push(' ');
        }
        let position = candidate.first_roll() + offset;
        if position > rolls.len() {
            cells.push('○');
        } else if prior_rolls_hidden && position != latest {
            cells.push('•');
        } else {
            cells.push(char::from(rolls.faces()[position - 1].ascii()));
        }
    }
    cells.push(']');
    cells
}
