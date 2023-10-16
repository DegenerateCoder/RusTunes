use crate::music_player::tui::TuiState;
use commands_registry::{Action, Arg, CommandDefinition, CommandType, CommandsRegistry};
use crossterm::event::KeyCode;
use std::collections::hash_map::HashMap;

pub mod commands_registry;

pub struct TuiCommands {
    global_commands: CommandsRegistry,
    history_state_commands: CommandsRegistry,
    global_commands_keys: HashMap<KeyCode, String>,
    state_commands_keys: HashMap<TuiState, HashMap<KeyCode, String>>,
}

impl TuiCommands {
    pub fn new() -> Self {
        let global_commands = Self::init_global_commands();
        let history_state_commands = Self::init_history_state_commands();
        let global_commands_keys = Self::init_global_commands_keys();
        let state_commands_keys =
            HashMap::from([(TuiState::History, Self::init_history_state_commands_keys())]);
        Self {
            global_commands,
            history_state_commands,
            global_commands_keys,
            state_commands_keys,
        }
    }

    pub fn map_keycode_to_action(&self, key: KeyCode, tui_state: &TuiState) -> Option<Action> {
        let mut global_command = true;
        let command_with_args = self.global_commands_keys.get(&key).or_else(|| {
            global_command = false;
            self.state_commands_keys
                .get(&tui_state)
                .and_then(|commands| commands.get(&key))
        });

        if global_command {
            return self
                .global_commands
                .map_command_str_to_action(command_with_args?);
        }

        match tui_state {
            TuiState::Player => None,
            TuiState::History => self
                .history_state_commands
                .map_command_str_to_action(command_with_args?),
        }
    }

    fn init_global_commands() -> CommandsRegistry {
        let mut global_commands = CommandsRegistry::new();
        global_commands.add_commands([
            CommandDefinition {
                name: "view".to_string(),
                action_type: CommandType::View,
                args: vec![Arg::TuiState(None)],
            },
            CommandDefinition {
                name: "quit".to_string(),
                action_type: CommandType::Quit,
                args: vec![],
            },
            CommandDefinition {
                name: "vol".to_string(),
                action_type: CommandType::Vol,
                args: vec![Arg::I64(None)],
            },
            CommandDefinition {
                name: "player-pause-resume".to_string(),
                action_type: CommandType::PlayerPauseResume,
                args: vec![],
            },
            CommandDefinition {
                name: "player-next".to_string(),
                action_type: CommandType::PlayerNext,
                args: vec![],
            },
            CommandDefinition {
                name: "player-prev".to_string(),
                action_type: CommandType::PlayerPrev,
                args: vec![],
            },
        ]);

        global_commands
    }

    fn init_global_commands_keys() -> HashMap<KeyCode, String> {
        HashMap::from([
            (KeyCode::Char('1'), "view player".to_string()),
            (KeyCode::Char('2'), "view history".to_string()),
            (KeyCode::Char('q'), "quit".to_string()),
            (KeyCode::Char('{'), "vol -1".to_string()),
            (KeyCode::Char('}'), "vol +1".to_string()),
            (KeyCode::Char('['), "vol -10".to_string()),
            (KeyCode::Char(']'), "vol +10".to_string()),
            (KeyCode::Char(' '), "player-pause-resume".to_string()),
            (KeyCode::Char('b'), "player-next".to_string()),
            (KeyCode::Char('z'), "player-prev".to_string()),
        ])
    }

    fn init_history_state_commands() -> CommandsRegistry {
        let mut history_state_commands = CommandsRegistry::new();
        history_state_commands.add_commands([CommandDefinition {
            name: "scroll".to_string(),
            action_type: CommandType::Scroll,
            args: vec![Arg::I16(None)],
        }]);

        history_state_commands
    }

    fn init_history_state_commands_keys() -> HashMap<KeyCode, String> {
        HashMap::from([
            (KeyCode::Char('j'), "scroll +1".to_string()),
            (KeyCode::Char('k'), "scroll -1".to_string()),
        ])
    }
}
