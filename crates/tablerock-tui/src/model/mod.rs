//! Root-owned terminal presentation state.

pub mod editor;
pub mod profiles;

use termrock::{
    Theme,
    input::{KeyCode, KeyEvent, KeyModifiers},
    interaction::{FocusOutcome, FocusRing},
    keymap::Keymap,
};

use crate::{ShellGeometry, ShellKeyAction, default_keymap, effect::RequestToken};
use editor::ConnectionFormModel;
use profiles::ProfileListState;

pub const MINIMUM_WIDTH: u16 = 40;
pub const MINIMUM_HEIGHT: u16 = 10;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutMode {
    Wide,
    Medium,
    Narrow,
    TooSmall,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusRegion {
    Context,
    Catalog,
    Tabs,
    Content,
    Actions,
    Footer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FocusScope {
    Shell,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionId {
    Open,
    New,
    Save,
    Test,
    Connect,
    Disconnect,
    Cancel,
    Quit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Connections,
    ConnectionPicker,
    /// First-version connection editor (new/edit).
    Editor,
    /// Stub workbench after Connect (session facts only until plan 007).
    Workbench,
}

/// Live session facts projected into the stub workbench.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SessionFacts {
    pub session_id_hex: String,
    pub identity: String,
    pub temporary: bool,
    pub engine_label: String,
    pub status: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShellTarget {
    Focus(FocusRegion),
    Action(ActionId),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScrollDirection {
    Up,
    Down,
    Left,
    Right,
}

impl FocusRegion {
    pub(crate) const CONNECTION_ORDER: [Self; 6] = [
        Self::Context,
        Self::Catalog,
        Self::Tabs,
        Self::Content,
        Self::Actions,
        Self::Footer,
    ];

    pub(crate) const PICKER_ORDER: [Self; 5] = [
        Self::Context,
        Self::Tabs,
        Self::Content,
        Self::Actions,
        Self::Footer,
    ];

    pub(crate) const EDITOR_ORDER: [Self; 4] =
        [Self::Context, Self::Content, Self::Actions, Self::Footer];

    pub(crate) const WORKBENCH_ORDER: [Self; 4] =
        [Self::Context, Self::Content, Self::Actions, Self::Footer];
}

#[derive(Debug)]
pub struct Model {
    pub(crate) theme: Theme,
    keymap: Keymap<ShellKeyAction>,
    width: u16,
    height: u16,
    focus: FocusRing<FocusRegion, FocusScope>,
    action: ActionId,
    screen: Screen,
    terminal_focused: bool,
    hovered: Option<ShellTarget>,
    pressed: Option<ShellTarget>,
    engine_resync_required: bool,
    /// Monotonic effect correlation counter (no clocks).
    next_request_token: RequestToken,
    profiles: ProfileListState,
    editor: ConnectionFormModel,
    session: Option<SessionFacts>,
    bootstrapped: bool,
}

impl Default for Model {
    fn default() -> Self {
        Self {
            theme: Theme::default(),
            keymap: default_keymap(),
            width: 0,
            height: 0,
            focus: initial_focus_ring(),
            action: ActionId::Open,
            screen: Screen::Connections,
            terminal_focused: true,
            hovered: None,
            pressed: None,
            engine_resync_required: false,
            next_request_token: 1,
            profiles: ProfileListState::default(),
            editor: ConnectionFormModel::default(),
            session: None,
            bootstrapped: false,
        }
    }
}

impl Model {
    #[must_use]
    pub const fn keymap(&self) -> &Keymap<ShellKeyAction> {
        &self.keymap
    }

    pub fn keymap_mut(&mut self) -> &mut Keymap<ShellKeyAction> {
        &mut self.keymap
    }

    #[must_use]
    pub const fn size(&self) -> (u16, u16) {
        (self.width, self.height)
    }

    #[must_use]
    pub const fn focus(&self) -> Option<FocusRegion> {
        self.focus.focused().copied()
    }

    #[must_use]
    pub const fn selected_action(&self) -> ActionId {
        self.action
    }

    #[must_use]
    pub const fn screen(&self) -> Screen {
        self.screen
    }

    #[must_use]
    pub const fn terminal_focused(&self) -> bool {
        self.terminal_focused
    }

    #[must_use]
    pub const fn hovered(&self) -> Option<ShellTarget> {
        self.hovered
    }

    #[must_use]
    pub const fn pressed(&self) -> Option<ShellTarget> {
        self.pressed
    }

    #[must_use]
    pub const fn engine_resync_required(&self) -> bool {
        self.engine_resync_required
    }

    #[must_use]
    pub const fn profiles(&self) -> &ProfileListState {
        &self.profiles
    }

    #[must_use]
    pub const fn bootstrapped(&self) -> bool {
        self.bootstrapped
    }

    #[must_use]
    pub const fn layout_mode(&self) -> LayoutMode {
        if self.width < MINIMUM_WIDTH || self.height < MINIMUM_HEIGHT {
            LayoutMode::TooSmall
        } else if self.width >= 100 {
            LayoutMode::Wide
        } else if self.width >= 64 {
            LayoutMode::Medium
        } else {
            LayoutMode::Narrow
        }
    }

    pub(crate) const fn resize(&mut self, width: u16, height: u16) {
        self.width = width;
        self.height = height;
    }

    pub(crate) fn request_focus(&mut self, focus: FocusRegion) -> bool {
        matches!(
            self.focus.request_focus(focus),
            FocusOutcome::Changed { .. }
        )
    }

    pub(crate) fn move_focus(&mut self, reverse: bool) -> bool {
        let key = KeyEvent::new(
            if reverse {
                KeyCode::BackTab
            } else {
                KeyCode::Tab
            },
            if reverse {
                KeyModifiers::SHIFT
            } else {
                KeyModifiers::NONE
            },
        );
        matches!(self.focus.handle_key(key), FocusOutcome::Changed { .. })
    }

    pub(crate) fn reconcile_focus_frame(&mut self, geometry: &ShellGeometry) -> bool {
        self.focus.begin_frame();
        let order: &[FocusRegion] = match self.screen {
            Screen::Connections => &FocusRegion::CONNECTION_ORDER,
            Screen::ConnectionPicker => &FocusRegion::PICKER_ORDER,
            Screen::Editor => &FocusRegion::EDITOR_ORDER,
            Screen::Workbench => &FocusRegion::WORKBENCH_ORDER,
        };
        let enabled = self.layout_mode() != LayoutMode::TooSmall;
        self.focus.register_order(
            FocusScope::Shell,
            order
                .iter()
                .copied()
                .map(|id| (id, geometry.focus_area(id), enabled)),
        );
        matches!(self.focus.reconcile(), FocusOutcome::Changed { .. })
    }

    pub(crate) const fn set_action(&mut self, action: ActionId) {
        self.action = action;
    }

    pub(crate) const fn set_screen(&mut self, screen: Screen) {
        self.screen = screen;
    }

    pub(crate) const fn set_terminal_focused(&mut self, focused: bool) {
        self.terminal_focused = focused;
        if !focused {
            self.hovered = None;
            self.pressed = None;
        }
    }

    pub(crate) const fn set_hovered(&mut self, target: Option<ShellTarget>) {
        self.hovered = target;
    }

    pub(crate) const fn set_pressed(&mut self, target: Option<ShellTarget>) {
        self.pressed = target;
    }

    pub(crate) const fn set_engine_resync_required(&mut self, required: bool) {
        self.engine_resync_required = required;
    }

    pub(crate) fn mint_request_token(&mut self) -> RequestToken {
        let token = self.next_request_token;
        self.next_request_token = self.next_request_token.saturating_add(1);
        token
    }

    pub(crate) fn set_profiles(&mut self, state: ProfileListState) {
        self.profiles = state;
    }

    pub(crate) fn profiles_mut(&mut self) -> &mut ProfileListState {
        &mut self.profiles
    }

    #[must_use]
    pub const fn editor(&self) -> &ConnectionFormModel {
        &self.editor
    }

    pub(crate) fn editor_mut(&mut self) -> &mut ConnectionFormModel {
        &mut self.editor
    }

    pub(crate) fn reset_editor(&mut self) {
        self.editor = ConnectionFormModel::default();
    }

    #[must_use]
    pub const fn session(&self) -> Option<&SessionFacts> {
        self.session.as_ref()
    }

    pub(crate) fn set_session(&mut self, session: Option<SessionFacts>) {
        self.session = session;
    }

    pub(crate) const fn set_bootstrapped(&mut self, value: bool) {
        self.bootstrapped = value;
    }
}

fn initial_focus_ring() -> FocusRing<FocusRegion, FocusScope> {
    let mut focus = FocusRing::new(FocusScope::Shell, Some(FocusRegion::Context));
    focus.begin_frame();
    focus.register_order(
        FocusScope::Shell,
        FocusRegion::CONNECTION_ORDER.map(|id| (id, None, true)),
    );
    let _ = focus.reconcile();
    focus
}
