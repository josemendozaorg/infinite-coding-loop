
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AppMode {
    MainMenu,
    Setup,
    Running,
    Marketplace,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MenuAction {
    NewGame,
    LoadGame,
    OpenMarketplace,
    Quit,
}

impl Default for AppMode {
    fn default() -> Self {
        AppMode::MainMenu
    }
}

pub struct MenuState {
    pub selected_index: usize,
    pub items: Vec<MenuAction>,
}

impl MenuState {
    pub fn new() -> Self {
        Self {
            selected_index: 0,
            items: vec![
                MenuAction::NewGame,
                MenuAction::LoadGame,
                MenuAction::OpenMarketplace,
                MenuAction::Quit,
            ],
        }
    }

    pub fn next(&mut self) {
        if self.selected_index < self.items.len() - 1 {
            self.selected_index += 1;
        } else {
            self.selected_index = 0;
        }
    }

    pub fn previous(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
        } else {
            self.selected_index = self.items.len() - 1;
        }
    }

    pub fn current_action(&self) -> MenuAction {
        self.items[self.selected_index].clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_menu_navigation() {
        let mut menu = MenuState::new();
        assert_eq!(menu.current_action(), MenuAction::NewGame);

        menu.next();
        assert_eq!(menu.current_action(), MenuAction::LoadGame);

        menu.next();
        menu.next();
        assert_eq!(menu.current_action(), MenuAction::Quit);

        menu.next(); // Wrap around
        assert_eq!(menu.current_action(), MenuAction::NewGame);

        menu.previous(); // Wrap back
        assert_eq!(menu.current_action(), MenuAction::Quit);
    }
}
