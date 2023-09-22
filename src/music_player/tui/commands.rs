use crate::music_player::tui::TuiState;
use crossterm::event::KeyCode;
use std::collections::hash_map::HashMap;

pub struct Command {
    pub name: String,
    pub action: Action,
}

#[derive(Clone)]
pub enum Action {
    ViewPlayer,
    ViewHistory,
    Quit,
    PlayerPauseResume,
    PlayerNext,
    PlayerPrev,
    Vol(i64),
    Scroll(i16),
}

pub struct TuiCommands {
    global_commands: HashMap<KeyCode, Command>,
    state_commands: HashMap<TuiState, HashMap<KeyCode, Command>>,
}

impl TuiCommands {
    pub fn new() -> Self {
        let global_commands = Self::init_global_commands();
        let state_commands = HashMap::from([(TuiState::History, Self::init_history_commands())]);
        Self {
            global_commands,
            state_commands,
        }
    }

    pub fn map_keycode_to_action(&self, key: KeyCode, tui_state: &TuiState) -> Option<Action> {
        let command = self.global_commands.get(&key).or(self
            .state_commands
            .get(&tui_state)
            .and_then(|commands| commands.get(&key)));

        Some(command?.action.clone())
    }

    fn init_global_commands() -> HashMap<KeyCode, Command> {
        HashMap::from([
            (
                KeyCode::Char('1'),
                Command {
                    name: "view player".to_string(),
                    action: Action::ViewPlayer,
                },
            ),
            (
                KeyCode::Char('2'),
                Command {
                    name: "view history".to_string(),
                    action: Action::ViewHistory,
                },
            ),
            (
                KeyCode::Char('q'),
                Command {
                    name: "quit".to_string(),
                    action: Action::Quit,
                },
            ),
            (
                KeyCode::Char('{'),
                Command {
                    name: "vol -1".to_string(),
                    action: Action::Vol(-1),
                },
            ),
            (
                KeyCode::Char('}'),
                Command {
                    name: "vol +1".to_string(),
                    action: Action::Vol(1),
                },
            ),
            (
                KeyCode::Char('['),
                Command {
                    name: "vol -10".to_string(),
                    action: Action::Vol(-10),
                },
            ),
            (
                KeyCode::Char(']'),
                Command {
                    name: "vol +10".to_string(),
                    action: Action::Vol(10),
                },
            ),
            (
                KeyCode::Char(' '),
                Command {
                    name: "player-pause-resume".to_string(),
                    action: Action::PlayerPauseResume,
                },
            ),
            (
                KeyCode::Char('b'),
                Command {
                    name: "player-next".to_string(),
                    action: Action::PlayerNext,
                },
            ),
            (
                KeyCode::Char('z'),
                Command {
                    name: "player-prev".to_string(),
                    action: Action::PlayerPrev,
                },
            ),
        ])
    }

    fn init_history_commands() -> HashMap<KeyCode, Command> {
        HashMap::from([
            (
                KeyCode::Char('j'),
                Command {
                    name: "view player".to_string(),
                    action: Action::Scroll(1),
                },
            ),
            (
                KeyCode::Char('k'),
                Command {
                    name: "view player".to_string(),
                    action: Action::Scroll(-1),
                },
            ),
        ])
    }
}
