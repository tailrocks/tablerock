//! Root-owned terminal presentation state.

use termrock::Theme;

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
pub enum ActionId {
    Open,
    Quit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Connections,
    ConnectionPicker,
}

impl FocusRegion {
    const ORDER: [Self; 6] = [
        Self::Context,
        Self::Catalog,
        Self::Tabs,
        Self::Content,
        Self::Actions,
        Self::Footer,
    ];

    #[must_use]
    pub fn next(self) -> Self {
        let index = Self::ORDER
            .iter()
            .position(|candidate| *candidate == self)
            .unwrap_or(0);
        Self::ORDER[(index + 1) % Self::ORDER.len()]
    }

    #[must_use]
    pub fn previous(self) -> Self {
        let index = Self::ORDER
            .iter()
            .position(|candidate| *candidate == self)
            .unwrap_or(0);
        Self::ORDER[(index + Self::ORDER.len() - 1) % Self::ORDER.len()]
    }
}

#[derive(Debug)]
pub struct Model {
    pub(crate) theme: Theme,
    width: u16,
    height: u16,
    focus: FocusRegion,
    action: ActionId,
    screen: Screen,
}

impl Default for Model {
    fn default() -> Self {
        Self {
            theme: Theme::default(),
            width: 0,
            height: 0,
            focus: FocusRegion::Context,
            action: ActionId::Open,
            screen: Screen::Connections,
        }
    }
}

impl Model {
    #[must_use]
    pub const fn size(&self) -> (u16, u16) {
        (self.width, self.height)
    }

    #[must_use]
    pub const fn focus(&self) -> FocusRegion {
        self.focus
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

    pub(crate) const fn set_focus(&mut self, focus: FocusRegion) {
        self.focus = focus;
    }

    pub(crate) const fn set_action(&mut self, action: ActionId) {
        self.action = action;
    }

    pub(crate) const fn set_screen(&mut self, screen: Screen) {
        self.screen = screen;
    }
}
