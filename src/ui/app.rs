use termion::event::Key;
pub(super) const SAFETY_ITEM_COUNT: usize = SafetyAttestation::ALL.len();
const ALL_SAFETY_CHECKS: u8 = (1 << SAFETY_ITEM_COUNT) - 1;

use crate::{
    application::{
        BackupSubmission, CeremonySession, ColdcardHashPreview, DerivationProjection, Generation,
        GroupSession, RollProgress, SafetyAttestation,
    },
    domain::{
        bip39::{BackupVerifier, EntropyTarget},
        ceremony::{Command, Phase},
        coin::CoinFlip,
        d20::D20Face,
        dice::DieFace,
        jade::{D8Face, D16Face},
        protocol::{
            BitBoxObservationKind, CoinFourD6ObservationKind, ConversionProtocol, JadeDieKind,
            bitbox_progress, coin_four_d6_progress, jade_expected_die,
        },
    },
    presentation::ProtocolMenuChoice,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InspectorView {
    Derivation,
    ProtocolExplanation,
    Help,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Inspector {
    pub view: InspectorView,
    pub scroll: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum UpdateOutcome {
    Unchanged,
    Changed,
    Exit,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum RollVisibility {
    LatestOnly,
    All,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum WordExactLedgerView {
    Assignments,
    Raw,
}

/// Which group sub-screen is showing. Pure view state — the domain service
/// does not know screens exist.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(super) enum GroupScreen {
    /// Collecting rolls into the current capture.
    #[default]
    Rolls,
    /// Showing the per-protocol accept/incomplete/invalid cards.
    Results,
}

/// Group-compare view state. `viewing` is which capture the Results screen is
/// browsing — a pure navigation cursor; it never determines a write target
/// (writes always extend the current/newest capture in the domain service).
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct GroupView {
    screen: GroupScreen,
    viewing: usize,
    revealed: bool,
    help: bool,
    /// Which group protocol's explanation is open, if any. An overlay over the
    /// current screen — a fixed index into [`crate::domain::group::GROUP_PROTOCOLS`].
    details: Option<usize>,
    /// Which accepted seed's derivation is open, if any — an index into the
    /// browsed capture's flattened accepted-seed list. Exposes secret material.
    derivation: Option<usize>,
}

#[derive(Eq, PartialEq)]
struct VisibleState {
    event_count: usize,
    generation_present: Option<()>,
    inspector: Option<Inspector>,
    quit_pending: bool,
    mnemonic_hidden: bool,
    verification: Option<(usize, usize)>,
    hidden_inspector: Option<Inspector>,
    message: Option<String>,
    target_cursor: usize,
    protocol_cursor: usize,
    roll_scroll: usize,
    roll_visibility: RollVisibility,
    word_exact_ledger_view: WordExactLedgerView,
    safety_cursor: usize,
    safety_checks: u8,
    group_captures: usize,
    group_roll_progress: Option<RollProgress>,
    group_view: GroupView,
}

pub struct App {
    session: CeremonySession,
    inspector: Option<Inspector>,
    quit_pending: bool,
    mnemonic_hidden: bool,
    verification: Option<BackupVerifier>,
    hidden_inspector: Option<Inspector>,
    message: Option<String>,
    target_cursor: usize,
    protocol_cursor: usize,
    roll_scroll: usize,
    roll_visibility: RollVisibility,
    word_exact_ledger_view: WordExactLedgerView,
    safety_cursor: usize,
    safety_checks: u8,
    group: Option<GroupSession>,
    group_view: GroupView,
    scroll_limit: usize,
}

impl App {
    #[must_use]
    pub fn new(session: CeremonySession) -> Self {
        Self {
            session,
            inspector: None,
            quit_pending: false,
            mnemonic_hidden: false,
            verification: None,
            hidden_inspector: None,
            message: None,
            target_cursor: 0,
            protocol_cursor: 0,
            roll_scroll: 0,
            roll_visibility: RollVisibility::LatestOnly,
            word_exact_ledger_view: WordExactLedgerView::Assignments,
            safety_cursor: 0,
            safety_checks: 0,
            group: None,
            group_view: GroupView::default(),
            scroll_limit: usize::MAX,
        }
    }
}

impl VisibleState {
    fn capture(app: &App) -> Self {
        Self {
            event_count: app.ceremony().events().len(),
            generation_present: app.generation().is_some().then_some(()),
            inspector: app.inspector,
            quit_pending: app.quit_pending,
            mnemonic_hidden: app.mnemonic_hidden,
            verification: app
                .verification
                .as_ref()
                .map(|verification| (verification.position(), verification.entry_len())),
            hidden_inspector: app.hidden_inspector,
            message: app.message.clone(),
            target_cursor: app.target_cursor,
            protocol_cursor: app.protocol_cursor,
            roll_scroll: app.roll_scroll,
            roll_visibility: app.roll_visibility,
            word_exact_ledger_view: app.word_exact_ledger_view,
            safety_cursor: app.safety_cursor,
            safety_checks: app.safety_checks,
            group_captures: app.group.as_ref().map_or(0, GroupSession::set_count),
            group_roll_progress: app.group.as_ref().map(GroupSession::roll_progress),
            group_view: app.group_view,
        }
    }
}

impl App {
    #[must_use]
    pub fn ceremony(&self) -> &crate::domain::ceremony::Ceremony {
        self.session.ceremony()
    }

    #[must_use]
    pub fn generation(&self) -> Option<&Generation> {
        self.session.generation()
    }

    #[must_use]
    pub fn coldcard_hash_preview(&self) -> Option<ColdcardHashPreview> {
        self.session.coldcard_hash_preview()
    }

    #[must_use]
    pub(super) const fn group(&self) -> Option<&GroupSession> {
        self.group.as_ref()
    }

    #[must_use]
    pub(super) const fn group_screen(&self) -> GroupScreen {
        self.group_view.screen
    }

    #[must_use]
    pub(super) const fn group_viewing(&self) -> usize {
        self.group_view.viewing
    }

    #[must_use]
    pub(super) const fn group_revealed(&self) -> bool {
        self.group_view.revealed
    }

    #[must_use]
    pub(super) const fn group_help(&self) -> bool {
        self.group_view.help
    }

    #[must_use]
    pub(super) const fn group_details(&self) -> Option<usize> {
        self.group_view.details
    }

    #[must_use]
    pub(super) const fn group_derivation(&self) -> Option<usize> {
        self.group_view.derivation
    }

    #[must_use]
    pub const fn inspector(&self) -> Option<Inspector> {
        self.inspector
    }

    #[must_use]
    pub const fn quit_pending(&self) -> bool {
        self.quit_pending
    }

    #[must_use]
    pub const fn mnemonic_hidden(&self) -> bool {
        self.mnemonic_hidden
    }

    #[must_use]
    pub fn mnemonic_verification(&self) -> Option<(usize, usize)> {
        self.verification
            .as_ref()
            .map(|verification| (verification.position(), verification.entry_len()))
    }

    #[must_use]
    pub fn message(&self) -> Option<&str> {
        self.message.as_deref()
    }

    #[must_use]
    pub const fn target_cursor(&self) -> usize {
        self.target_cursor
    }

    #[must_use]
    pub const fn protocol_cursor(&self) -> usize {
        self.protocol_cursor
    }

    #[must_use]
    pub(super) const fn selected_protocol_choice(&self) -> ProtocolMenuChoice {
        ProtocolMenuChoice::ALL[self.protocol_cursor]
    }

    #[must_use]
    pub(super) fn selected_protocol(&self) -> Option<ConversionProtocol> {
        let target = self.ceremony().state().target()?;
        self.selected_protocol_choice().implemented_protocol(target)
    }

    #[must_use]
    pub const fn roll_scroll(&self) -> usize {
        self.roll_scroll
    }

    #[must_use]
    pub const fn rolls_hidden(&self) -> bool {
        matches!(self.roll_visibility, RollVisibility::LatestOnly)
    }

    #[must_use]
    pub const fn word_exact_raw_ledger(&self) -> bool {
        matches!(self.word_exact_ledger_view, WordExactLedgerView::Raw)
    }

    #[must_use]
    pub const fn safety_cursor(&self) -> usize {
        self.safety_cursor
    }

    #[must_use]
    pub const fn safety_check_count(&self) -> usize {
        self.safety_checks.count_ones() as usize
    }

    #[must_use]
    pub const fn safety_checked(&self, index: usize) -> bool {
        self.safety_checks & (1 << index) != 0
    }

    #[must_use]
    pub const fn safety_all_checked(&self) -> bool {
        self.safety_checks == ALL_SAFETY_CHECKS
    }

    #[must_use]
    pub fn derivation_available(&self) -> bool {
        self.generation().is_some() && self.ceremony().state().phase() == Phase::Revealed
    }

    #[must_use]
    pub fn derivation(&self) -> Option<DerivationProjection> {
        if !self.derivation_available() {
            return None;
        }
        self.session.derivation(self.ceremony().events().len())
    }

    /// Handles one decoded terminal key and reports whether rendering is invalidated.
    #[cfg(test)]
    pub(super) fn update(&mut self, key: Key) -> UpdateOutcome {
        self.update_bounded(key, usize::MAX)
    }

    pub(super) fn update_bounded(&mut self, key: Key, scroll_limit: usize) -> UpdateOutcome {
        self.scroll_limit = scroll_limit;
        self.update_with_limit(key)
    }

    fn update_with_limit(&mut self, key: Key) -> UpdateOutcome {
        let before = VisibleState::capture(self);
        if !self.update_inner(key) {
            return UpdateOutcome::Exit;
        }
        if before == VisibleState::capture(self) {
            UpdateOutcome::Unchanged
        } else {
            UpdateOutcome::Changed
        }
    }

    fn update_inner(&mut self, key: Key) -> bool {
        self.message = None;
        if self.quit_pending {
            return self.update_quit_confirmation(key);
        }
        if self.verification.is_some() {
            return self.update_mnemonic_verification(key);
        }
        if self.mnemonic_hidden {
            return self.update_hidden_mnemonic(key);
        }
        if self.inspector.is_some() {
            self.update_inspector(key);
            return true;
        }
        if self.group.is_some() {
            return self.update_group(key);
        }
        if matches!(key, Key::Char('q') | Key::Ctrl('c')) {
            self.quit_pending = true;
            return true;
        }
        if matches!(key, Key::Char('?')) {
            self.open_inspector(InspectorView::Help);
            return true;
        }
        if matches!(key, Key::Char('d')) && self.derivation_available() {
            self.open_inspector(InspectorView::Derivation);
            return true;
        }
        if self.update_scroll(key) {
            return true;
        }

        self.update_phase(key);
        true
    }

    fn update_scroll(&mut self, key: Key) -> bool {
        self.roll_scroll = self.roll_scroll.min(self.scroll_limit);
        match key {
            Key::PageUp
                if self.ceremony().state().phase() == Phase::EnterRolls
                    && self.normal_capture_document_scroll() =>
            {
                self.roll_scroll = self.roll_scroll.saturating_sub(4);
            }
            Key::PageDown
                if self.ceremony().state().phase() == Phase::EnterRolls
                    && self.normal_capture_document_scroll() =>
            {
                self.roll_scroll = self.roll_scroll.saturating_add(4).min(self.scroll_limit);
            }
            Key::PageUp if self.ceremony().state().phase() == Phase::EnterRolls => {
                self.roll_scroll = self.roll_scroll.saturating_add(4).min(self.scroll_limit);
            }
            Key::PageDown if self.ceremony().state().phase() == Phase::EnterRolls => {
                self.roll_scroll = self.roll_scroll.saturating_sub(4);
            }
            Key::PageUp => self.roll_scroll = self.roll_scroll.saturating_sub(4),
            Key::PageDown => {
                self.roll_scroll = self.roll_scroll.saturating_add(4).min(self.scroll_limit);
            }
            _ => return false,
        }
        true
    }

    fn update_phase(&mut self, key: Key) {
        match self.ceremony().state().phase() {
            Phase::ChooseTarget => self.choose_target(key),
            Phase::ChooseProtocol => self.choose_protocol(key),
            Phase::Safety => self.acknowledge_safety(key),
            Phase::EnterRolls => self.enter_roll(key),
            Phase::AttemptRejected => self.restart_attempt(key),
            Phase::Result => self.reveal(key),
            Phase::Revealed => self.review_mnemonic(key),
            Phase::ReadyToGenerate | Phase::Cancelled => {}
        }
    }

    fn choose_target(&mut self, key: Key) {
        if let Some(direction) = choice_direction(key) {
            self.target_cursor = move_cursor(self.target_cursor, 2, direction);
        } else if matches!(key, Key::Char('\n' | 'l') | Key::Right) {
            let target = [EntropyTarget::Words12, EntropyTarget::Words24][self.target_cursor];
            self.handle(Command::SelectTarget(target));
        } else if matches!(key, Key::Char(_)) {
            self.message = Some("Use ↑/↓ to choose, then Enter to continue.".to_owned());
        }
    }

    fn choose_protocol(&mut self, key: Key) {
        if let Some(direction) = choice_direction(key) {
            self.protocol_cursor = move_cursor(
                self.protocol_cursor,
                ProtocolMenuChoice::ALL.len(),
                direction,
            );
            return;
        }
        match key {
            Key::Char('e') => self.open_inspector(InspectorView::ProtocolExplanation),
            Key::Char('h') | Key::Left => self.handle(Command::ReopenTargetSelection),
            Key::Char('\n' | 'l') | Key::Right => self.choose_selected_protocol(),
            Key::Char('g') => self.enter_group_mode(),
            Key::Char(_) => {
                self.message =
                    Some("Use ↑/↓ to choose, Enter to continue, ← to go back.".to_owned());
            }
            _ => {}
        }
    }

    fn enter_group_mode(&mut self) {
        let Some(target) = self.ceremony().state().target() else {
            return;
        };
        self.group = Some(GroupSession::new(target));
        self.group_view = GroupView::default();
        self.roll_scroll = 0;
    }

    fn choose_selected_protocol(&mut self) {
        if let Some(protocol) = self.selected_protocol() {
            self.handle(Command::SelectProtocol(protocol));
        } else {
            self.message = Some(format!(
                "{} does not support this mnemonic length. Press e for details.",
                self.selected_protocol_choice().name()
            ));
        }
    }

    fn acknowledge_safety(&mut self, key: Key) {
        if let Some(direction) = choice_direction(key) {
            self.safety_cursor = move_cursor(self.safety_cursor, SAFETY_ITEM_COUNT, direction);
            return;
        }
        match key {
            Key::Char(' ') => self.safety_checks ^= 1 << self.safety_cursor,
            Key::Char('c') => self.safety_checks = ALL_SAFETY_CHECKS,
            Key::Char('h') | Key::Left => self.leave_safety(),
            Key::Char('\n') => self.confirm_safety(),
            Key::Char(_) => {
                self.message = Some("Use ↑/↓, Space to check, or c to check all.".to_owned());
            }
            _ => {}
        }
    }

    fn leave_safety(&mut self) {
        self.safety_cursor = 0;
        self.safety_checks = 0;
        self.handle(Command::ReopenProtocolSelection);
    }

    fn confirm_safety(&mut self) {
        if self.safety_all_checked() {
            self.handle(Command::AcknowledgeSafety);
        } else {
            self.message = Some("Complete all safety checks before continuing.".to_owned());
        }
    }

    fn enter_roll(&mut self, key: Key) {
        if self.ceremony().state().protocol() == Some(ConversionProtocol::JadeDirectV1) {
            self.enter_jade_roll(key);
            return;
        }
        if self.ceremony().state().protocol() == Some(ConversionProtocol::BitBox02DirectV1) {
            self.enter_bitbox_observation(key);
            return;
        }
        if self.ceremony().state().protocol() == Some(ConversionProtocol::CoinFourD6DirectV1) {
            self.enter_coin_four_d6_observation(key);
            return;
        }
        if self.ceremony().state().protocol() == Some(ConversionProtocol::KruxD20V1) {
            self.enter_d20_roll(key);
            return;
        }
        if self.ceremony().state().protocol() == Some(ConversionProtocol::SeedSignerCoinsV1) {
            self.enter_flip(key);
            return;
        }
        match key {
            Key::Char(character @ '1'..='6') => {
                if let Ok(face) = DieFace::try_from(character) {
                    self.handle(Command::RecordRoll(face));
                }
            }
            Key::Backspace | Key::Delete => self.handle(Command::UndoRoll),
            _ if self.enter_capture_control(key) => {}
            Key::Char(_) => {
                self.message = Some("Only digits 1–6 are valid rolls.".to_owned());
            }
            _ => {}
        }
    }

    fn enter_jade_roll(&mut self, key: Key) {
        if matches!(key, Key::Backspace | Key::Delete) {
            self.handle(Command::UndoJade);
            return;
        }
        if self.enter_capture_control(key) {
            return;
        }
        let state = self.ceremony().state();
        let Some(target) = state.target() else { return };
        let Some(expected) = jade_expected_die(target, state.jade().len()) else {
            return;
        };
        match (expected, key) {
            (JadeDieKind::D16, Key::Char(character)) => {
                let value = match character {
                    '1'..='9' => character
                        .to_digit(10)
                        .and_then(|value| u8::try_from(value).ok()),
                    'A'..='G' => Some(10 + (character as u8 - b'A')),
                    _ => None,
                };
                if let Some(face) = value.and_then(|value| D16Face::new(value).ok()) {
                    self.handle(Command::RecordJadeD16(face));
                } else {
                    self.message =
                        Some("D16: use 1–9 or uppercase A–G for faces 10–16.".to_owned());
                }
            }
            (JadeDieKind::D8, Key::Char(character @ '1'..='8')) => {
                let value = character
                    .to_digit(10)
                    .and_then(|value| u8::try_from(value).ok());
                if let Some(face) = value.and_then(|value| D8Face::new(value).ok()) {
                    self.handle(Command::RecordJadeD8(face));
                }
            }
            (JadeDieKind::D8, Key::Char(_)) => {
                self.message = Some("D8: only faces 1–8 are valid.".to_owned());
            }
            _ => {}
        }
    }

    fn enter_bitbox_observation(&mut self, key: Key) {
        if matches!(key, Key::Backspace | Key::Delete) {
            self.handle(Command::UndoBitBox);
            return;
        }
        if self.enter_capture_control(key) {
            return;
        }
        let state = self.ceremony().state();
        let Some(target) = state.target() else { return };
        match (bitbox_progress(target, state.bitbox()).expected_kind(), key) {
            (Some(BitBoxObservationKind::D6), Key::Char(character @ '1'..='6')) => {
                if let Ok(face) = DieFace::try_from(character) {
                    self.handle(Command::RecordBitBoxD6(face));
                }
            }
            (Some(BitBoxObservationKind::D6), Key::Char(_)) => {
                self.message =
                    Some("D6: use 1–6; faces 5 and 6 are recorded then retried.".to_owned());
            }
            (Some(BitBoxObservationKind::Coin), Key::Char(character @ ('0' | '1'))) => {
                if let Ok(flip) = CoinFlip::try_from(character) {
                    self.handle(Command::RecordBitBoxCoin(flip));
                }
            }
            (Some(BitBoxObservationKind::Coin), Key::Char(_)) => {
                self.message = Some("Coin: use 0 for tails or 1 for heads.".to_owned());
            }
            _ => {}
        }
    }

    fn enter_coin_four_d6_observation(&mut self, key: Key) {
        if matches!(key, Key::Backspace | Key::Delete) {
            self.handle(Command::UndoCoinFourD6);
            return;
        }
        if self.enter_capture_control(key) {
            return;
        }
        let state = self.ceremony().state();
        let Some(target) = state.target() else { return };
        match (
            coin_four_d6_progress(target, state.coin_four_d6()).expected_kind(),
            key,
        ) {
            (Some(CoinFourD6ObservationKind::Coin), Key::Char(character @ ('0' | '1'))) => {
                if let Ok(flip) = CoinFlip::try_from(character) {
                    self.handle(Command::RecordCoinFourD6Coin(flip));
                }
            }
            (Some(CoinFourD6ObservationKind::Coin), Key::Char(_)) => {
                self.message = Some("Coin: use 0 for tails or 1 for heads.".to_owned());
            }
            (Some(CoinFourD6ObservationKind::D6), Key::Char(character @ '1'..='6')) => {
                if let Ok(face) = DieFace::try_from(character) {
                    self.handle(Command::RecordCoinFourD6D6(face));
                }
            }
            (Some(CoinFourD6ObservationKind::D6), Key::Char(_)) => {
                self.message = Some("D6: only faces 1–6 are valid.".to_owned());
            }
            _ => {}
        }
    }

    fn enter_d20_roll(&mut self, key: Key) {
        if matches!(key, Key::Backspace | Key::Delete) {
            self.handle(Command::UndoD20);
            return;
        }
        if self.enter_capture_control(key) {
            return;
        }
        if let Key::Char(character) = key {
            let value = match character {
                '1'..='9' => character
                    .to_digit(10)
                    .and_then(|value| u8::try_from(value).ok()),
                'A'..='K' => Some(10 + (character as u8 - b'A')),
                _ => None,
            };
            if let Some(face) = value.and_then(|value| D20Face::new(value).ok()) {
                self.handle(Command::RecordD20(face));
            } else {
                self.message = Some("D20: use 1–9 or uppercase A–K for faces 10–20.".to_owned());
            }
        }
    }

    fn enter_flip(&mut self, key: Key) {
        match key {
            Key::Char(character @ ('0' | '1')) => {
                if let Ok(flip) = CoinFlip::try_from(character) {
                    self.handle(Command::RecordFlip(flip));
                }
            }
            Key::Backspace | Key::Delete => self.handle(Command::UndoFlip),
            _ if self.enter_capture_control(key) => {}
            Key::Char(_) => {
                self.message = Some("Only 0 (tails) and 1 (heads) are valid flips.".to_owned());
            }
            _ => {}
        }
    }

    fn enter_capture_control(&mut self, key: Key) -> bool {
        match key {
            Key::Char('e') => self.open_inspector(InspectorView::ProtocolExplanation),
            Key::Char('h') => {
                self.roll_visibility = match self.roll_visibility {
                    RollVisibility::LatestOnly => RollVisibility::All,
                    RollVisibility::All => RollVisibility::LatestOnly,
                };
            }
            Key::Char('l')
                if self.ceremony().state().protocol() == Some(ConversionProtocol::WordExactV1) =>
            {
                self.word_exact_ledger_view = match self.word_exact_ledger_view {
                    WordExactLedgerView::Assignments => WordExactLedgerView::Raw,
                    WordExactLedgerView::Raw => WordExactLedgerView::Assignments,
                };
                self.roll_scroll = 0;
            }
            Key::Up if self.normal_capture_document_scroll() => {
                self.roll_scroll = self.roll_scroll.saturating_sub(1);
            }
            Key::Down if self.normal_capture_document_scroll() => {
                self.roll_scroll = self.roll_scroll.saturating_add(1).min(self.scroll_limit);
            }
            Key::PageUp if self.normal_capture_document_scroll() => {
                self.roll_scroll = self.roll_scroll.saturating_sub(4);
            }
            Key::PageDown if self.normal_capture_document_scroll() => {
                self.roll_scroll = self.roll_scroll.saturating_add(4).min(self.scroll_limit);
            }
            Key::Up => {
                self.roll_scroll = self.roll_scroll.saturating_add(1).min(self.scroll_limit);
            }
            Key::Down => self.roll_scroll = self.roll_scroll.saturating_sub(1),
            Key::PageUp => {
                self.roll_scroll = self.roll_scroll.saturating_add(4).min(self.scroll_limit);
            }
            Key::PageDown => self.roll_scroll = self.roll_scroll.saturating_sub(4),
            Key::Char('\n') => {
                if let Err(error) = self.session.confirm_rolls_and_generate() {
                    self.message = Some(error.to_string());
                }
            }
            _ => return false,
        }
        true
    }

    fn normal_capture_document_scroll(&self) -> bool {
        self.word_exact_assignments_active()
            || matches!(
                self.ceremony().state().protocol(),
                Some(
                    ConversionProtocol::JadeDirectV1
                        | ConversionProtocol::BitBox02DirectV1
                        | ConversionProtocol::KruxD20V1
                        | ConversionProtocol::CoinFourD6DirectV1
                )
            )
    }

    fn word_exact_assignments_active(&self) -> bool {
        self.ceremony().state().protocol() == Some(ConversionProtocol::WordExactV1)
            && !self.word_exact_raw_ledger()
    }

    fn update_group(&mut self, key: Key) -> bool {
        if matches!(key, Key::Char('q') | Key::Ctrl('c')) {
            self.quit_pending = true;
            return true;
        }
        if self.group_view.help {
            if matches!(key, Key::Char('?') | Key::Esc) {
                self.group_view.help = false;
            }
            return true;
        }
        if self.group_view.details.is_some() {
            self.group_browse_details(key);
            return true;
        }
        if self.group_view.derivation.is_some() {
            self.group_browse_derivation(key);
            return true;
        }
        if matches!(key, Key::Char('?')) {
            self.group_view.help = true;
            return true;
        }
        if matches!(key, Key::Char('e')) {
            self.group_view.details = Some(0);
            self.roll_scroll = 0;
            return true;
        }
        match self.group_view.screen {
            GroupScreen::Rolls => self.group_enter_roll(key),
            GroupScreen::Results => self.group_review(key),
        }
        true
    }

    /// Keys while a protocol-details overlay is open: cycle protocols, scroll,
    /// or close back to the underlying screen.
    fn group_browse_details(&mut self, key: Key) {
        let count = crate::domain::group::GROUP_PROTOCOLS.len();
        match key {
            Key::Char('e') | Key::Esc => {
                self.group_view.details = None;
                self.roll_scroll = 0;
            }
            Key::Left => {
                self.group_view.details = self.group_view.details.map(|i| (i + count - 1) % count);
                self.roll_scroll = 0;
            }
            Key::Right => {
                self.group_view.details = self.group_view.details.map(|i| (i + 1) % count);
                self.roll_scroll = 0;
            }
            Key::Up => self.roll_scroll = self.roll_scroll.saturating_sub(1),
            Key::Down => {
                self.roll_scroll = self.roll_scroll.saturating_add(1).min(self.scroll_limit);
            }
            Key::PageUp => self.roll_scroll = self.roll_scroll.saturating_sub(4),
            Key::PageDown => {
                self.roll_scroll = self.roll_scroll.saturating_add(4).min(self.scroll_limit);
            }
            _ => {}
        }
    }

    /// Keys while a derivation overlay is open: step through accepted seeds,
    /// scroll, or close back to the results screen.
    fn group_browse_derivation(&mut self, key: Key) {
        let count = self.group_accepted_count().max(1);
        match key {
            Key::Char('d') | Key::Esc => {
                self.group_view.derivation = None;
                self.roll_scroll = 0;
            }
            Key::Left => {
                self.group_view.derivation =
                    self.group_view.derivation.map(|i| (i + count - 1) % count);
                self.roll_scroll = 0;
            }
            Key::Right => {
                self.group_view.derivation = self.group_view.derivation.map(|i| (i + 1) % count);
                self.roll_scroll = 0;
            }
            Key::Up => self.roll_scroll = self.roll_scroll.saturating_sub(1),
            Key::Down => {
                self.roll_scroll = self.roll_scroll.saturating_add(1).min(self.scroll_limit);
            }
            Key::PageUp => self.roll_scroll = self.roll_scroll.saturating_sub(4),
            Key::PageDown => {
                self.roll_scroll = self.roll_scroll.saturating_add(4).min(self.scroll_limit);
            }
            _ => {}
        }
    }

    /// Accepted-seed count for the capture the results screen is browsing.
    fn group_accepted_count(&self) -> usize {
        self.group
            .as_ref()
            .map_or(0, |session| session.accepted_count(self.group_view.viewing))
    }

    fn group_enter_roll(&mut self, key: Key) {
        match key {
            Key::Char(character @ '1'..='6') => {
                if let Ok(face) = DieFace::try_from(character)
                    && let Some(session) = self.group.as_mut()
                {
                    session.record_roll(face);
                }
            }
            Key::Backspace | Key::Delete => {
                if let Some(session) = self.group.as_mut() {
                    session.undo_roll();
                }
            }
            Key::Char('\n') => {
                // Show the capture just built: browse starts on the current one.
                self.group_view.revealed = false;
                self.group_view.screen = GroupScreen::Results;
                self.group_view.viewing = self.group_current_index();
                self.roll_scroll = 0;
            }
            Key::Up => self.roll_scroll = self.roll_scroll.saturating_sub(1),
            Key::Down => {
                self.roll_scroll = self.roll_scroll.saturating_add(1).min(self.scroll_limit);
            }
            Key::PageUp => self.roll_scroll = self.roll_scroll.saturating_sub(4),
            Key::PageDown => {
                self.roll_scroll = self.roll_scroll.saturating_add(4).min(self.scroll_limit);
            }
            Key::Char(_) => {
                self.message = Some("Only digits 1–6 are valid rolls.".to_owned());
            }
            _ => {}
        }
    }

    /// Index of the current (newest) capture — the write target.
    fn group_current_index(&self) -> usize {
        self.group
            .as_ref()
            .map_or(0, |session| session.set_count().saturating_sub(1))
    }

    fn group_review(&mut self, key: Key) {
        match key {
            Key::Char('c') => {
                self.group_view.screen = GroupScreen::Rolls;
                self.roll_scroll = 0;
            }
            Key::Char('n') => {
                self.group_view.revealed = false;
                self.group_view.screen = GroupScreen::Rolls;
                self.roll_scroll = 0;
                if let Some(session) = self.group.as_mut() {
                    session.start_fresh_set();
                }
                self.group_view.viewing = self.group_current_index();
            }
            Key::Char('r') => self.group_view.revealed = !self.group_view.revealed,
            Key::Char('d') => {
                if self.group_accepted_count() > 0 {
                    self.group_view.derivation = Some(0);
                    self.roll_scroll = 0;
                } else {
                    self.message =
                        Some("No accepted seed to derive in this capture yet.".to_owned());
                }
            }
            Key::Left | Key::Char('h') => {
                self.roll_scroll = 0;
                self.group_view.revealed = false;
                self.group_view.viewing = self.group_view.viewing.saturating_sub(1);
            }
            Key::Right | Key::Char('l') => {
                self.roll_scroll = 0;
                self.group_view.revealed = false;
                let last = self.group_current_index();
                self.group_view.viewing = (self.group_view.viewing + 1).min(last);
            }
            Key::Up => self.roll_scroll = self.roll_scroll.saturating_sub(1),
            Key::Down => {
                self.roll_scroll = self.roll_scroll.saturating_add(1).min(self.scroll_limit);
            }
            Key::PageUp => self.roll_scroll = self.roll_scroll.saturating_sub(4),
            Key::PageDown => {
                self.roll_scroll = self.roll_scroll.saturating_add(4).min(self.scroll_limit);
            }
            _ => {}
        }
    }

    fn restart_attempt(&mut self, key: Key) {
        if matches!(key, Key::Char('\n' | 'r')) {
            self.handle(Command::RestartAttempt);
        }
    }

    fn reveal(&mut self, key: Key) {
        if matches!(key, Key::Char('r')) {
            self.handle(Command::RevealMnemonic);
        }
    }

    fn review_mnemonic(&mut self, key: Key) {
        if matches!(key, Key::Char('h')) {
            self.hidden_inspector = None;
            self.mnemonic_hidden = true;
        } else if matches!(key, Key::Char('v')) {
            self.verification = Some(BackupVerifier::new());
        } else if matches!(key, Key::Char('\n'))
            && self.ceremony().state().mnemonic_backup_verified()
        {
            self.quit_pending = true;
        }
    }

    fn update_mnemonic_verification(&mut self, key: Key) -> bool {
        match key {
            Key::Esc => self.verification = None,
            Key::Ctrl('c') => {
                self.verification = None;
                self.quit_pending = true;
            }
            Key::Backspace => {
                if let Some(verification) = &mut self.verification {
                    verification.pop();
                }
            }
            Key::Char('\n') => self.check_mnemonic_word(),
            Key::Char(character) if character.is_ascii_lowercase() => {
                if !self
                    .verification
                    .as_mut()
                    .is_some_and(|verification| verification.push(character))
                {
                    self.message =
                        Some("Backup words use at most 16 lowercase letters.".to_owned());
                }
            }
            _ => {
                self.message = Some("Type lowercase word letters, then press Enter.".to_owned());
            }
        }
        true
    }

    fn check_mnemonic_word(&mut self) {
        let Some(mut verification) = self.verification.take() else {
            return;
        };
        match self.session.submit_backup_word(&mut verification) {
            Ok(BackupSubmission::Mismatch { position }) => {
                self.message = Some(format!(
                    "Position {:02} did not match. Check the backup and try again.",
                    position + 1
                ));
                self.verification = Some(verification);
            }
            Ok(BackupSubmission::Next { .. }) => self.verification = Some(verification),
            Ok(BackupSubmission::Complete) => {}
            Err(error) => self.message = Some(error.to_string()),
        }
    }

    fn update_hidden_mnemonic(&mut self, key: Key) -> bool {
        match key {
            Key::Char('h') | Key::Esc => {
                self.mnemonic_hidden = false;
                self.inspector = self.hidden_inspector.take();
            }
            Key::Char('q') | Key::Ctrl('c') => self.quit_pending = true,
            _ => {}
        }
        true
    }

    fn update_inspector(&mut self, key: Key) {
        let Some(mut inspector) = self.inspector else {
            return;
        };
        inspector.scroll = inspector.scroll.min(self.scroll_limit);
        if inspector.view == InspectorView::ProtocolExplanation {
            match key {
                Key::Char('q') | Key::Ctrl('c') => {
                    self.inspector = None;
                    self.quit_pending = true;
                }
                Key::Esc | Key::Char('e' | '\t') => self.inspector = None,
                Key::Up => inspector.scroll = inspector.scroll.saturating_sub(1),
                Key::Down => {
                    inspector.scroll = inspector.scroll.saturating_add(1).min(self.scroll_limit);
                    self.inspector = Some(inspector);
                }
                Key::PageUp => inspector.scroll = inspector.scroll.saturating_sub(8),
                Key::PageDown => {
                    inspector.scroll = inspector.scroll.saturating_add(8).min(self.scroll_limit);
                    self.inspector = Some(inspector);
                }
                _ => self.inspector = Some(inspector),
            }
            if self.inspector.is_some() {
                self.inspector = Some(inspector);
            }
            return;
        }
        match key {
            Key::Char('q') | Key::Ctrl('c') => {
                self.inspector = None;
                self.quit_pending = true;
                return;
            }
            Key::Char('h') if inspector.view == InspectorView::Derivation => {
                self.hidden_inspector = Some(inspector);
                self.inspector = None;
                self.mnemonic_hidden = true;
                return;
            }
            Key::Esc | Key::Char('\t') => self.inspector = None,
            Key::Up => inspector.scroll = inspector.scroll.saturating_sub(1),
            Key::Down => {
                inspector.scroll = inspector.scroll.saturating_add(1).min(self.scroll_limit);
            }
            Key::PageUp => inspector.scroll = inspector.scroll.saturating_sub(8),
            Key::PageDown => {
                inspector.scroll = inspector.scroll.saturating_add(8).min(self.scroll_limit);
            }
            Key::Char('d') if self.derivation_available() => {
                inspector.view = InspectorView::Derivation;
                inspector.scroll = 0;
            }
            Key::Char('?') => {
                inspector.view = InspectorView::Help;
                inspector.scroll = 0;
            }
            _ => {}
        }
        if self.inspector.is_some() {
            self.inspector = Some(inspector);
        }
    }

    fn open_inspector(&mut self, view: InspectorView) {
        self.inspector = Some(Inspector { view, scroll: 0 });
    }

    fn update_quit_confirmation(&mut self, key: Key) -> bool {
        match key {
            Key::Char('q' | '\n') | Key::Ctrl('c') => {
                let _ = self.session.cancel();
                false
            }
            Key::Esc | Key::Char('n') => {
                self.quit_pending = false;
                true
            }
            _ => true,
        }
    }

    fn handle(&mut self, command: Command) {
        if let Err(error) = self.session.execute(command) {
            self.message = Some(error.to_string());
        }
    }
}

fn choice_direction(key: Key) -> Option<isize> {
    match key {
        Key::Up | Key::Char('k') => Some(-1),
        Key::Down | Key::Char('j') => Some(1),
        _ => None,
    }
}

fn move_cursor(current: usize, length: usize, direction: isize) -> usize {
    if direction < 0 {
        current.checked_sub(1).unwrap_or(length - 1)
    } else {
        (current + 1) % length
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use zeroize::Zeroizing;

    fn enter_safety(app: &mut App) {
        app.update(Key::Char('\n'));
        app.update(Key::Char('\n'));
    }

    fn configure(app: &mut App, mode: ConversionProtocol) {
        app.update(Key::Char('\n'));
        let steps = match mode {
            ConversionProtocol::ColdcardV1 => 0,
            ConversionProtocol::WordExactV1 => 1,
            ConversionProtocol::ExactV1 => 2,
            ConversionProtocol::JadeDirectV1 => 4,
            ConversionProtocol::BitBox02DirectV1 => 5,
            ConversionProtocol::KruxD20V1 => 6,
            ConversionProtocol::CoinFourD6DirectV1 => 7,
            ConversionProtocol::SeedSignerCoinsV1 => 8,
            ConversionProtocol::KeystoneLegacyV1 => unreachable!(),
        };
        for _ in 0..steps {
            app.update(Key::Char('j'));
        }
        app.update(Key::Char('\n'));
        app.update(Key::Char('c'));
        app.update(Key::Char('\n'));
    }

    fn enter_bitbox_zero_remainder(app: &mut App) {
        for _ in 1..11 {
            for _ in 0..5 {
                app.update(Key::Char('1'));
            }
            app.update(Key::Char('1'));
        }
        for _ in 0..7 {
            app.update(Key::Char('1'));
        }
    }

    fn verify_mnemonic(app: &mut App) {
        let words = Zeroizing::new(app.generation().unwrap().mnemonic().words().to_vec());
        app.update(Key::Char('v'));
        for word in words.iter() {
            for character in word.chars() {
                app.update(Key::Char(character));
            }
            app.update(Key::Char('\n'));
        }
    }

    #[test]
    fn keys_drive_semantic_state_transitions() {
        let mut app = App::default();
        configure(&mut app, ConversionProtocol::ExactV1);
        assert_eq!(app.ceremony().state().phase(), Phase::EnterRolls);

        app.update(Key::Char('4'));
        app.update(Key::Backspace);
        assert!(app.ceremony().state().rolls().is_empty());
        assert_eq!(app.ceremony().events().len(), 5);
    }

    #[test]
    fn state_inspection_shortcuts_are_ignored_across_ceremony_stages() {
        let mut app = App::default();
        app.update(Key::Char('i'));
        assert!(app.inspector().is_none());

        configure(&mut app, ConversionProtocol::ExactV1);
        for _ in 0..50 {
            app.update(Key::Char('4'));
        }
        let events = app.ceremony().events().len();

        app.update(Key::Char('i'));
        app.update(Key::Char('\t'));
        assert!(app.inspector().is_none());
        assert_eq!(app.ceremony().events().len(), events);

        app.update(Key::Char('\n'));
        app.update(Key::Char('i'));
        assert!(app.inspector().is_none());

        app.update(Key::Char('r'));
        app.update(Key::Char('i'));
        assert!(app.inspector().is_none());
    }

    #[test]
    fn word_exact_assignment_and_raw_ledger_toggle_without_domain_events() {
        let mut app = App::default();
        configure(&mut app, ConversionProtocol::WordExactV1);
        let events = app.ceremony().events().len();

        app.update(Key::Char('l'));
        assert!(app.word_exact_raw_ledger());
        assert_eq!(app.ceremony().events().len(), events);

        app.update(Key::Char('l'));
        assert!(!app.word_exact_raw_ledger());
        assert_eq!(app.ceremony().events().len(), events);
    }

    #[test]
    fn word_exact_assignment_scroll_uses_normal_document_directions() {
        let mut app = App::default();
        configure(&mut app, ConversionProtocol::WordExactV1);

        assert_eq!(app.update_bounded(Key::Down, 5), UpdateOutcome::Changed);
        assert_eq!(app.roll_scroll(), 1);
        app.update_bounded(Key::PageDown, 5);
        assert_eq!(app.roll_scroll(), 5);
        assert_eq!(
            app.update_bounded(Key::PageDown, 5),
            UpdateOutcome::Unchanged
        );
        app.update_bounded(Key::Up, 5);
        assert_eq!(app.roll_scroll(), 4);
        app.update_bounded(Key::PageUp, 5);
        assert_eq!(app.roll_scroll(), 0);
    }

    #[test]
    fn coin_four_d6_scroll_uses_normal_document_directions() {
        let mut app = App::default();
        configure(&mut app, ConversionProtocol::CoinFourD6DirectV1);

        app.update_bounded(Key::Down, 5);
        assert_eq!(app.roll_scroll(), 1);
        app.update_bounded(Key::Up, 5);
        assert_eq!(app.roll_scroll(), 0);
    }

    #[test]
    fn roll_card_scroll_does_not_create_domain_events() {
        let mut app = App::default();
        configure(&mut app, ConversionProtocol::ExactV1);
        let events = app.ceremony().events().len();

        app.update(Key::Up);
        app.update(Key::PageUp);
        assert_eq!(app.roll_scroll(), 5);
        app.update(Key::Down);
        assert_eq!(app.roll_scroll(), 4);
        assert_eq!(app.ceremony().events().len(), events);
    }

    #[test]
    fn quit_requires_confirmation() {
        let mut app = App::default();
        assert_eq!(app.update(Key::Char('q')), UpdateOutcome::Changed);
        assert!(app.quit_pending());
        assert_eq!(app.update(Key::Esc), UpdateOutcome::Changed);
        assert!(!app.quit_pending());

        assert_eq!(app.update(Key::Char('q')), UpdateOutcome::Changed);
        assert_eq!(app.update(Key::Char('q')), UpdateOutcome::Exit);
        assert_eq!(app.ceremony().state().phase(), Phase::Cancelled);
    }

    #[test]
    fn invalid_menu_key_surfaces_a_hint() {
        let mut app = App::default();
        app.update(Key::Char('1'));
        assert!(app.message().is_some());
        assert!(app.ceremony().events().is_empty());
    }

    #[test]
    fn invalid_roll_key_surfaces_a_hint() {
        let mut app = App::default();
        configure(&mut app, ConversionProtocol::ExactV1);
        app.update(Key::Char('7'));
        assert_eq!(app.message(), Some("Only digits 1–6 are valid rolls."));
        assert!(app.ceremony().state().rolls().is_empty());
    }

    #[test]
    fn invalid_keys_do_not_create_domain_events() {
        let mut app = App::default();
        assert_eq!(app.update(Key::Char('1')), UpdateOutcome::Changed);
        assert_eq!(app.update(Key::Char('x')), UpdateOutcome::Unchanged);
        assert!(app.ceremony().events().is_empty());
    }

    #[test]
    fn non_action_key_does_not_invalidate_rendering() {
        let mut app = App::default();
        assert_eq!(app.update(Key::F(1)), UpdateOutcome::Unchanged);

        configure(&mut app, ConversionProtocol::ExactV1);
        assert_eq!(app.update(Key::F(1)), UpdateOutcome::Unchanged);
    }

    #[test]
    fn menus_support_choice_and_step_navigation() {
        let mut app = App::default();
        app.update(Key::Up);
        assert_eq!(app.target_cursor(), 1);
        app.update(Key::Char('j'));
        assert_eq!(app.target_cursor(), 0);
        app.update(Key::Down);
        app.update(Key::Right);
        assert_eq!(
            app.ceremony().state().target(),
            Some(EntropyTarget::Words24)
        );

        app.update(Key::Char('j'));
        app.update(Key::Down);
        assert_eq!(app.protocol_cursor(), 2);
        app.update(Key::Left);
        assert_eq!(app.ceremony().state().phase(), Phase::ChooseTarget);
        assert_eq!(app.ceremony().state().target(), None);

        app.update(Key::Char('l'));
        app.update(Key::Right);
        assert_eq!(
            app.ceremony().state().protocol(),
            Some(ConversionProtocol::ExactV1)
        );
    }

    #[test]
    fn unsupported_target_explains_but_cannot_advance() {
        let mut app = App::default();
        app.update(Key::Char('\n'));
        for _ in 0..3 {
            app.update(Key::Down);
        }
        assert_eq!(
            app.selected_protocol_choice(),
            ProtocolMenuChoice::KeystoneLegacyDice
        );

        let events = app.ceremony().events().len();
        app.update(Key::Char('\n'));
        assert_eq!(app.ceremony().state().phase(), Phase::ChooseProtocol);
        assert_eq!(app.ceremony().events().len(), events);
        assert!(
            app.message()
                .is_some_and(|message| message.contains("does not support"))
        );

        app.update(Key::Char('e'));
        assert_eq!(
            app.inspector().map(|inspector| inspector.view),
            Some(InspectorView::ProtocolExplanation)
        );
    }

    #[test]
    fn jade_mixed_dice_capture_accepts_expected_die_and_generates() {
        let mut app = App::default();
        configure(&mut app, ConversionProtocol::JadeDirectV1);

        app.update(Key::Char('0'));
        assert!(app.message().is_some_and(|message| message.contains("D16")));
        app.update(Key::Char('A'));
        assert_eq!(app.ceremony().state().jade().observations()[0].face(), 10);
        app.update(Key::Backspace);

        for _ in 0..35 {
            app.update(Key::Char('1'));
        }
        assert_eq!(app.ceremony().state().jade().len(), 35);
        app.update(Key::Char('\n'));
        assert_eq!(app.ceremony().state().phase(), Phase::Result);
        assert_eq!(app.generation().unwrap().mnemonic().words()[11], "about");
    }

    #[test]
    fn bitbox_capture_handles_local_rejection_kind_switch_and_generation() {
        let mut app = App::default();
        configure(&mut app, ConversionProtocol::BitBox02DirectV1);

        let events = app.ceremony().events().len();
        app.update(Key::Char('0'));
        assert_eq!(app.ceremony().events().len(), events);
        assert!(app.message().is_some_and(|message| message.contains("D6")));

        app.update(Key::Char('6'));
        for _ in 0..5 {
            app.update(Key::Char('1'));
        }
        let events = app.ceremony().events().len();
        app.update(Key::Char('2'));
        assert_eq!(app.ceremony().events().len(), events);
        assert!(
            app.message()
                .is_some_and(|message| message.contains("Coin"))
        );
        app.update(Key::Char('1'));
        app.update(Key::Backspace);
        app.update(Key::Char('1'));

        enter_bitbox_zero_remainder(&mut app);
        assert_eq!(app.ceremony().state().bitbox().len(), 74);
        app.update(Key::Char('\n'));
        assert_eq!(app.ceremony().state().phase(), Phase::Result);
        assert_eq!(app.generation().unwrap().mnemonic().words()[0], "abandon");
    }

    #[test]
    fn krux_d20_capture_accepts_encoded_faces_extras_and_generates() {
        let mut app = App::default();
        configure(&mut app, ConversionProtocol::KruxD20V1);

        app.update(Key::Char('0'));
        assert!(app.message().is_some_and(|message| message.contains("D20")));
        app.update(Key::Char('A'));
        assert_eq!(app.ceremony().state().d20().faces()[0].get(), 10);
        app.update(Key::Backspace);
        for _ in 0..30 {
            app.update(Key::Char('1'));
        }
        assert!(app.ceremony().state().can_confirm_rolls());
        app.update(Key::Char('K'));
        assert_eq!(app.ceremony().state().d20().faces()[30].get(), 20);
        app.update(Key::Char('\n'));
        assert_eq!(app.ceremony().state().phase(), Phase::Result);
    }

    #[test]
    fn coin_four_d6_capture_enforces_kinds_and_generates() {
        let mut app = App::default();
        configure(&mut app, ConversionProtocol::CoinFourD6DirectV1);

        app.update(Key::Char('2'));
        assert!(
            app.message()
                .is_some_and(|message| message.contains("Coin"))
        );
        app.update(Key::Char('1'));
        app.update(Key::Char('0'));
        assert!(app.message().is_some_and(|message| message.contains("D6")));
        for _ in 0..4 {
            app.update(Key::Char('1'));
        }
        for _ in 1..12 {
            app.update(Key::Char('1'));
            for _ in 0..4 {
                app.update(Key::Char('1'));
            }
        }
        assert_eq!(app.ceremony().state().coin_four_d6().len(), 60);
        assert!(app.ceremony().state().can_confirm_rolls());
        app.update(Key::Char('\n'));
        assert_eq!(app.ceremony().state().phase(), Phase::Result);
        assert_eq!(app.generation().unwrap().mnemonic().words()[0], "abandon");
    }

    #[test]
    fn seedsigner_coin_capture_accepts_only_bits_and_generates() {
        let mut app = App::default();
        configure(&mut app, ConversionProtocol::SeedSignerCoinsV1);

        app.update(Key::Char('2'));
        assert_eq!(
            app.message(),
            Some("Only 0 (tails) and 1 (heads) are valid flips.")
        );
        for _ in 0..128 {
            app.update(Key::Char('0'));
        }
        assert_eq!(app.ceremony().state().flips().len(), 128);
        assert!(app.ceremony().state().rolls().is_empty());
        app.update(Key::Char('\n'));
        assert_eq!(app.ceremony().state().phase(), Phase::Result);
    }

    #[test]
    fn keystone_legacy_is_selectable_only_for_24_words() {
        let mut app = App::default();
        app.update(Key::Down);
        app.update(Key::Char('\n'));
        for _ in 0..3 {
            app.update(Key::Down);
        }

        assert_eq!(
            app.selected_protocol(),
            Some(ConversionProtocol::KeystoneLegacyV1)
        );
        app.update(Key::Char('\n'));
        assert_eq!(app.ceremony().state().phase(), Phase::Safety);
    }

    #[test]
    fn selected_protocol_explanation_opens_and_returns_without_domain_events() {
        let mut app = App::default();
        app.update(Key::Char('\n'));
        let events = app.ceremony().events().len();

        assert_eq!(app.update(Key::Char('e')), UpdateOutcome::Changed);
        assert_eq!(
            app.inspector().map(|inspector| inspector.view),
            Some(InspectorView::ProtocolExplanation)
        );
        assert_eq!(app.ceremony().events().len(), events);
        assert_eq!(app.update(Key::Right), UpdateOutcome::Unchanged);
        assert_eq!(app.update(Key::Down), UpdateOutcome::Changed);
        assert_eq!(app.inspector().map(|inspector| inspector.scroll), Some(1));
        assert_eq!(app.update(Key::Char('\t')), UpdateOutcome::Changed);
        assert!(app.inspector().is_none());
    }

    #[test]
    fn roll_capture_protocol_explanation_preserves_live_state() {
        let mut app = App::default();
        configure(&mut app, ConversionProtocol::ExactV1);
        app.update(Key::Char('4'));
        let events = app.ceremony().events().len();

        app.update(Key::Char('e'));
        assert_eq!(
            app.inspector().map(|inspector| inspector.view),
            Some(InspectorView::ProtocolExplanation)
        );
        assert_eq!(app.ceremony().events().len(), events);

        app.update(Key::Esc);
        assert!(app.inspector().is_none());
        assert_eq!(app.ceremony().state().phase(), Phase::EnterRolls);
        assert_eq!(app.ceremony().state().rolls().len(), 1);
    }

    #[test]
    fn protocol_detail_has_one_safe_back_level() {
        let mut app = App::default();
        app.update(Key::Char('\n'));
        app.update(Key::Char('e'));

        assert_eq!(app.update(Key::Esc), UpdateOutcome::Changed);
        assert!(app.inspector().is_none());
        assert_eq!(app.update(Key::Esc), UpdateOutcome::Unchanged);
        assert!(!app.quit_pending());
    }

    #[test]
    fn safety_checks_are_interactive_ui_state() {
        let mut app = App::default();
        enter_safety(&mut app);
        let events = app.ceremony().events().len();

        app.update(Key::Char(' '));
        assert!(app.safety_checked(0));
        app.update(Key::Down);
        app.update(Key::Char(' '));
        assert!(app.safety_checked(1));
        assert_eq!(app.safety_check_count(), 2);
        assert_eq!(app.ceremony().events().len(), events);
    }

    #[test]
    fn safety_checks_gate_aggregate_acknowledgement() {
        let mut app = App::default();
        enter_safety(&mut app);
        let events = app.ceremony().events().len();

        app.update(Key::Char('\n'));
        assert_eq!(app.ceremony().state().phase(), Phase::Safety);
        assert_eq!(
            app.message(),
            Some("Complete all safety checks before continuing.")
        );
        app.update(Key::Char('c'));
        assert!(app.safety_all_checked());
        app.update(Key::Char('\n'));
        assert_eq!(app.ceremony().state().phase(), Phase::EnterRolls);
        assert_eq!(app.ceremony().events().len(), events + 1);
    }

    #[test]
    fn safety_can_return_to_the_preserved_protocol_cursor() {
        let mut app = App::default();
        app.update(Key::Char('\n'));
        app.update(Key::Char('j'));
        app.update(Key::Char('\n'));
        assert_eq!(app.ceremony().state().phase(), Phase::Safety);

        app.update(Key::Char('h'));
        assert_eq!(app.ceremony().state().phase(), Phase::ChooseProtocol);
        assert_eq!(app.ceremony().state().protocol(), None);
        assert_eq!(app.protocol_cursor(), 1);
    }

    #[test]
    fn protocol_detail_scroll_stops_at_the_end_and_reverses_immediately() {
        let mut app = App::default();
        app.update(Key::Char('\n'));
        app.update(Key::Char('e'));

        app.update_bounded(Key::PageDown, 3);
        assert_eq!(app.inspector().map(|inspector| inspector.scroll), Some(3));
        assert_eq!(
            app.update_bounded(Key::PageDown, 3),
            UpdateOutcome::Unchanged
        );
        assert_eq!(app.inspector().map(|inspector| inspector.scroll), Some(3));
        assert_eq!(app.update_bounded(Key::Up, 3), UpdateOutcome::Changed);
        assert_eq!(app.inspector().map(|inspector| inspector.scroll), Some(2));
    }

    #[test]
    fn bounded_inspector_scroll_stops_at_the_end_and_reverses_immediately() {
        let mut app = App::default();
        app.update(Key::Char('?'));

        app.update_bounded(Key::PageDown, 5);
        assert_eq!(app.inspector().unwrap().scroll, 5);
        assert_eq!(app.update_bounded(Key::Down, 5), UpdateOutcome::Unchanged);
        assert_eq!(app.update_bounded(Key::Up, 5), UpdateOutcome::Changed);
        assert_eq!(app.inspector().unwrap().scroll, 4);
    }

    #[test]
    fn help_content_scrolls_without_changing_state() {
        let mut app = App::default();
        app.update(Key::Char('?'));
        app.update(Key::PageDown);
        app.update(Key::Down);
        assert_eq!(app.inspector().unwrap().scroll, 9);
        assert!(app.ceremony().events().is_empty());

        app.update(Key::Up);
        assert_eq!(app.inspector().unwrap().scroll, 8);
    }

    #[test]
    fn reveal_can_be_quick_hidden_without_changing_domain_state() {
        let mut app = App::default();
        configure(&mut app, ConversionProtocol::ExactV1);
        for _ in 0..50 {
            app.update(Key::Char('1'));
        }
        app.update(Key::Char('\n'));
        app.update(Key::Char('r'));
        let events = app.ceremony().events().len();

        app.update(Key::Char('h'));
        assert!(app.mnemonic_hidden());
        assert_eq!(app.ceremony().events().len(), events);
        app.update(Key::Char('i'));
        assert!(app.inspector().is_none());
        app.update(Key::Esc);
        assert!(!app.mnemonic_hidden());
    }

    #[test]
    fn mnemonic_check_enables_enter_to_finish() {
        let mut app = App::default();
        configure(&mut app, ConversionProtocol::ExactV1);
        for _ in 0..50 {
            app.update(Key::Char('1'));
        }
        app.update(Key::Char('\n'));
        app.update(Key::Char('r'));

        app.update(Key::Char('\n'));
        assert!(!app.quit_pending());
        app.update(Key::Char('v'));
        assert_eq!(app.mnemonic_verification(), Some((0, 0)));
        app.update(Key::Char('z'));
        app.update(Key::Char('\n'));
        assert_eq!(app.mnemonic_verification(), Some((0, 0)));
        assert!(app.message().unwrap().contains("Position 01 did not match"));
        assert!(!app.ceremony().state().mnemonic_backup_verified());

        app.update(Key::Esc);
        verify_mnemonic(&mut app);
        assert!(app.ceremony().state().mnemonic_backup_verified());
        app.update(Key::Char('\n'));
        assert!(app.quit_pending());
    }

    #[test]
    fn concealed_result_requires_the_explicit_reveal_key() {
        let mut app = App::default();
        configure(&mut app, ConversionProtocol::ExactV1);
        for _ in 0..50 {
            app.update(Key::Char('1'));
        }
        app.update(Key::Char('\n'));
        let events = app.ceremony().events().len();

        assert_eq!(app.ceremony().state().phase(), Phase::Result);
        assert_eq!(app.update(Key::Char('\n')), UpdateOutcome::Unchanged);
        assert_eq!(app.ceremony().state().phase(), Phase::Result);
        assert_eq!(app.ceremony().events().len(), events);
        app.update(Key::Char('r'));
        assert_eq!(app.ceremony().state().phase(), Phase::Revealed);
    }

    #[test]
    fn derivation_shortcut_opens_only_after_reveal() {
        let mut app = App::default();
        configure(&mut app, ConversionProtocol::ExactV1);
        for _ in 0..50 {
            app.update(Key::Char('1'));
        }
        app.update(Key::Char('\n'));
        assert_eq!(app.ceremony().state().phase(), Phase::Result);

        app.update(Key::Char('d'));
        assert!(app.inspector().is_none());
        assert!(!app.derivation_available());

        app.update(Key::Char('r'));
        app.update(Key::Char('d'));
        assert!(app.derivation_available());
        assert_eq!(app.inspector().unwrap().view, InspectorView::Derivation);
    }

    #[test]
    fn derivation_can_quick_hide_all_secrets_without_a_domain_event() {
        let mut app = App::default();
        configure(&mut app, ConversionProtocol::ExactV1);
        for _ in 0..50 {
            app.update(Key::Char('1'));
        }
        app.update(Key::Char('\n'));
        app.update(Key::Char('r'));
        app.update(Key::Char('d'));
        let events = app.ceremony().events().len();

        app.update(Key::Char('h'));

        assert!(app.mnemonic_hidden());
        assert!(app.inspector().is_none());
        assert_eq!(app.ceremony().events().len(), events);

        app.update(Key::Char('h'));

        assert!(!app.mnemonic_hidden());
        assert_eq!(app.inspector().unwrap().view, InspectorView::Derivation);
        assert_eq!(app.ceremony().events().len(), events);
    }

    fn group_app() -> App {
        let mut app = App::default();
        app.update(Key::Down); // 24 words
        app.update(Key::Char('\n'));
        app.update(Key::Char('g'));
        app
    }

    fn group_push_fair(app: &mut App, count: usize) {
        for index in 0..count {
            let face = u8::try_from((index % 6) + 1).expect("1..=6 fits u8");
            app.update(Key::Char(char::from(b'0' + face)));
        }
    }

    #[test]
    fn group_mode_starts_from_the_protocol_screen_and_creates_no_domain_events() {
        let mut app = App::default();
        app.update(Key::Down);
        app.update(Key::Char('\n'));
        let events = app.ceremony().events().len();

        assert!(app.group().is_none());
        app.update(Key::Char('g'));
        assert!(app.group().is_some());
        // Group compare lives in the app layer; the ceremony aggregate is untouched.
        assert_eq!(app.ceremony().events().len(), events);
        assert_eq!(app.ceremony().state().phase(), Phase::ChooseProtocol);
    }

    #[test]
    fn group_collects_rolls_then_enter_switches_to_results() {
        let mut app = group_app();
        group_push_fair(&mut app, 100);
        assert_eq!(app.group().unwrap().roll_progress().recorded, 100);
        assert_eq!(app.group_screen(), GroupScreen::Rolls);

        app.update(Key::Char('\n'));
        assert_eq!(app.group_screen(), GroupScreen::Results);

        // Fair 100-roll tape: every protocol in the 100 set accepts.
        let report = app.group().unwrap().comparison_at(0);
        let set = report.sets().iter().find(|s| s.rolls == 100).unwrap();
        assert!(set.protocols.iter().all(|(_, s)| s.calculation().is_some()));
    }

    #[test]
    fn group_undo_removes_the_latest_roll() {
        let mut app = group_app();
        group_push_fair(&mut app, 3);
        app.update(Key::Backspace);
        assert_eq!(app.group().unwrap().roll_progress().recorded, 2);
    }

    #[test]
    fn group_invalid_set_is_only_cleared_by_a_fresh_capture() {
        let mut app = group_app();
        for _ in 0..100 {
            app.update(Key::Char('6')); // all sixes: exact out-of-range and coldcard >30%
        }
        app.update(Key::Char('\n'));
        let report = app.group().unwrap().comparison_at(0);
        let set = report.sets().iter().find(|s| s.rolls == 100).unwrap();
        assert!(set.protocols.iter().any(|(_, status)| status.is_rejected()));

        app.update(Key::Char('n'));
        assert_eq!(app.group().unwrap().set_count(), 2);
        assert_eq!(app.group_viewing(), 1);
        assert_eq!(app.group().unwrap().roll_progress().recorded, 0);
        assert_eq!(app.group_screen(), GroupScreen::Rolls);
    }

    #[test]
    fn group_reveal_toggles_without_touching_the_tape() {
        let mut app = group_app();
        group_push_fair(&mut app, 100);
        app.update(Key::Char('\n'));
        assert!(!app.group_revealed());

        app.update(Key::Char('r'));
        assert!(app.group_revealed());
        assert_eq!(app.group().unwrap().roll_progress().recorded, 100);

        // Switching captures re-conceals for safety.
        app.update(Key::Char('n'));
        assert!(!app.group_revealed());
    }

    #[test]
    fn group_quit_still_requires_confirmation() {
        let mut app = group_app();
        assert_eq!(app.update(Key::Char('q')), UpdateOutcome::Changed);
        assert!(app.quit_pending());
    }

    #[test]
    fn group_help_toggles_without_disturbing_the_tape() {
        let mut app = group_app();
        group_push_fair(&mut app, 12);

        app.update(Key::Char('?'));
        assert!(app.group_help());
        // Rolls are ignored while help is open, and the tape is preserved.
        app.update(Key::Char('1'));
        assert_eq!(app.group().unwrap().roll_progress().recorded, 12);

        app.update(Key::Esc);
        assert!(!app.group_help());
        app.update(Key::Char('1'));
        assert_eq!(app.group().unwrap().roll_progress().recorded, 13);
    }

    #[test]
    fn group_details_cycles_the_protocols_and_preserves_the_tape() {
        let mut app = group_app();
        group_push_fair(&mut app, 8);

        // [e] opens the details overlay on the first group protocol.
        app.update(Key::Char('e'));
        assert_eq!(app.group_details(), Some(0));

        // Arrows step through every group protocol and wrap around.
        let count = crate::domain::group::GROUP_PROTOCOLS.len();
        app.update(Key::Right);
        assert_eq!(app.group_details(), Some(1));
        app.update(Key::Left);
        app.update(Key::Left);
        assert_eq!(app.group_details(), Some(count - 1));

        // Digits are inert while details are open; the tape is untouched.
        app.update(Key::Char('1'));
        assert_eq!(app.group().unwrap().roll_progress().recorded, 8);

        // Esc closes back to the collect screen and rolling resumes.
        app.update(Key::Esc);
        assert_eq!(app.group_details(), None);
        assert_eq!(app.group_screen(), GroupScreen::Rolls);
        app.update(Key::Char('1'));
        assert_eq!(app.group().unwrap().roll_progress().recorded, 9);
    }

    #[test]
    fn group_derivation_steps_through_accepted_seeds_and_exposes_the_secret() {
        let mut app = group_app();
        group_push_fair(&mut app, 100); // fair tape: several protocols accept
        app.update(Key::Char('\n')); // results

        let accepted = app.group().unwrap().accepted_count(0);
        assert!(accepted > 1, "fair 100-roll tape accepts several protocols");

        // [d] opens the derivation overlay on the first accepted seed.
        app.update(Key::Char('d'));
        assert_eq!(app.group_derivation(), Some(0));

        // Arrows step through every accepted seed and wrap.
        app.update(Key::Right);
        assert_eq!(app.group_derivation(), Some(1));
        app.update(Key::Left);
        app.update(Key::Left);
        assert_eq!(app.group_derivation(), Some(accepted - 1));

        // Esc closes back to results.
        app.update(Key::Esc);
        assert_eq!(app.group_derivation(), None);
        assert_eq!(app.group_screen(), GroupScreen::Results);
    }

    #[test]
    fn group_derivation_needs_an_accepted_seed() {
        let mut app = group_app();
        group_push_fair(&mut app, 20); // too few rolls: no protocol completes
        app.update(Key::Char('\n')); // results
        assert_eq!(app.group().unwrap().accepted_count(0), 0);

        app.update(Key::Char('d'));
        assert_eq!(app.group_derivation(), None);
        assert!(app.message().is_some());
    }

    #[test]
    fn group_details_open_from_results_and_return_there() {
        let mut app = group_app();
        group_push_fair(&mut app, 100);
        app.update(Key::Char('\n')); // results
        assert_eq!(app.group_screen(), GroupScreen::Results);

        app.update(Key::Char('e'));
        assert_eq!(app.group_details(), Some(0));
        app.update(Key::Char('e')); // toggle closed
        assert_eq!(app.group_details(), None);
        assert_eq!(app.group_screen(), GroupScreen::Results);
    }

    #[test]
    fn group_browsing_is_view_only_and_writes_target_the_current_capture() {
        let mut app = group_app();
        group_push_fair(&mut app, 5); // capture 0: 5 rolls
        app.update(Key::Char('\n')); // results
        app.update(Key::Char('n')); // fresh capture 1 (current), back to rolls
        group_push_fair(&mut app, 3); // capture 1: 3 rolls
        assert_eq!(app.group().unwrap().set_count(), 2);
        assert_eq!(app.group().unwrap().roll_progress().recorded, 3);

        app.update(Key::Char('\n')); // results, viewing starts on the current capture
        assert_eq!(app.group_viewing(), 1);
        app.update(Key::Left); // browse to the older capture
        assert_eq!(app.group_viewing(), 0);
        // Browsing moved no write target: capture 0 frozen, current still capture 1.
        assert_eq!(
            app.group()
                .unwrap()
                .comparison_at(0)
                .sets()
                .first()
                .unwrap()
                .rolls,
            5
        );
        assert_eq!(app.group().unwrap().roll_progress().recorded, 3);

        // Rolling more continues the current capture, not the browsed one.
        app.update(Key::Char('c'));
        group_push_fair(&mut app, 2);
        assert_eq!(app.group().unwrap().roll_progress().recorded, 5);
        assert_eq!(
            app.group()
                .unwrap()
                .comparison_at(0)
                .sets()
                .first()
                .unwrap()
                .rolls,
            5
        );
    }
}
