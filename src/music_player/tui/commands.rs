use crate::music_player::tui::TuiState;
use commands_registry::{Arg, CommandAction, CommandDefinition, CommandType, CommandsRegistry};
use crossterm::event::KeyCode;
use std::collections::hash_map::HashMap;
use std::fmt::Write;

pub mod commands_registry;

pub struct TuiCommands {
    global_commands: CommandsRegistry,
    states_commands: HashMap<TuiState, CommandsRegistry>,
    global_commands_keys: HashMap<KeyCode, String>,
    states_commands_keys: HashMap<TuiState, HashMap<KeyCode, String>>,
}

impl TuiCommands {
    pub fn new() -> Self {
        let global_commands = Self::init_global_commands();
        let states_commands = HashMap::from([
            (TuiState::History, Self::init_history_state_commands()),
            (TuiState::Help, Self::init_help_state_commands()),
        ]);
        let global_commands_keys = Self::init_global_commands_keys();
        let states_commands_keys = HashMap::from([
            (TuiState::History, Self::init_history_state_commands_keys()),
            (TuiState::Help, Self::init_help_state_commands_keys()),
        ]);
        Self {
            global_commands,
            states_commands,
            global_commands_keys,
            states_commands_keys,
        }
    }

    pub fn map_command_text_to_action(
        &self,
        command_text: &str,
        tui_state: &TuiState,
    ) -> Option<CommandAction> {
        let mut action = self.global_commands.map_command_str_to_action(command_text);

        if action.is_none() {
            action = self
                .states_commands
                .get(tui_state)
                .map(|state_commands| state_commands.map_command_str_to_action(command_text))?;
        }

        action
    }

    pub fn map_keycode_to_action(
        &self,
        key: KeyCode,
        tui_state: &TuiState,
    ) -> Option<CommandAction> {
        let mut global_command = true;
        let command_with_args = self.global_commands_keys.get(&key).or_else(|| {
            global_command = false;
            self.states_commands_keys
                .get(&tui_state)
                .and_then(|commands| commands.get(&key))
        });

        if global_command {
            return self
                .global_commands
                .map_command_str_to_action(command_with_args?);
        }

        self.states_commands
            .get(&tui_state)
            .and_then(|state_commands| state_commands.map_command_str_to_action(command_with_args?))
    }

    pub fn generate_suggestions(&self, command_text: &str, tui_state: &TuiState) -> Vec<String> {
        let global_command_names = self.global_commands.get_commands_names();
        let states_commands_names = self
            .states_commands
            .get(&tui_state)
            .map(|state_comands| state_comands.get_commands_names());

        let mut commands_suggestions = Vec::new();
        for command_name in global_command_names {
            if let Some(ins_dist) = Self::calculate_ins_distance(command_text, command_name) {
                commands_suggestions.push((command_name, ins_dist));
            }
        }
        if let Some(states_commands_names) = states_commands_names {
            for command_name in states_commands_names {
                if let Some(ins_dist) = Self::calculate_ins_distance(command_text, command_name) {
                    commands_suggestions.push((command_name, ins_dist));
                }
            }
        }

        commands_suggestions.sort_by_key(|(_, inst_dist)| *inst_dist);

        commands_suggestions
            .iter()
            .map(|(command_name, _)| (**command_name).to_owned())
            .collect()
    }

    pub fn calculate_ins_distance(from: &str, to: &str) -> Option<u8> {
        if !to.starts_with(from) {
            return None;
        }

        let insertions = to.chars().skip(from.chars().count());

        Some(insertions.count().try_into().unwrap())
    }

    pub fn generate_help_str(&self) -> String {
        let mut help_str = String::new();

        writeln!(help_str, "Commands:").unwrap();

        let min_width = 7;

        let global_help_str = self.global_commands.generate_help_str();
        global_help_str
            .lines()
            .for_each(|command| writeln!(help_str, "{:min_width$}  {command}", "global").unwrap());

        let mut states: Vec<&TuiState> = self.states_commands.keys().collect();
        states.sort_by_key(|state| state.to_str());

        for state in states {
            let state_commands = self.states_commands.get(state);
            if let Some(state_commands) = state_commands {
                let state_help_str = state_commands.generate_help_str();
                state_help_str.lines().for_each(|command| {
                    writeln!(help_str, "{:min_width$}  {command}", state.to_str()).unwrap()
                });
            }
        }

        writeln!(help_str).unwrap();
        writeln!(help_str, "Keybindings:").unwrap();

        let global_keys_help_str = Self::generate_keys_help_str(&self.global_commands_keys);
        global_keys_help_str
            .lines()
            .for_each(|command| writeln!(help_str, "{:min_width$}  {command}", "global").unwrap());

        let mut states: Vec<&TuiState> = self.states_commands_keys.keys().collect();
        states.sort_by_key(|state| state.to_str());

        for state in states {
            let keys_help_str =
                Self::generate_keys_help_str(self.states_commands_keys.get(state).unwrap());
            keys_help_str.lines().for_each(|command| {
                writeln!(help_str, "{:min_width$}  {command}", state.to_str()).unwrap()
            });
        }

        help_str
    }

    fn init_global_commands() -> CommandsRegistry {
        let mut global_commands = CommandsRegistry::new();
        global_commands.add_commands([
            CommandDefinition {
                name: "enter_command_mode".to_string(),
                action_type: CommandType::EnterCommandMode,
                args: vec![],
                user_facing: false,
            },
            CommandDefinition {
                name: "view".to_string(),
                action_type: CommandType::View,
                args: vec![Arg::TUISTATE(None)],
                user_facing: true,
            },
            CommandDefinition {
                name: "quit".to_string(),
                action_type: CommandType::Quit,
                args: vec![],
                user_facing: true,
            },
            CommandDefinition {
                name: "vol".to_string(),
                action_type: CommandType::Vol,
                args: vec![Arg::I64(None)],
                user_facing: true,
            },
            CommandDefinition {
                name: "player-pause-resume".to_string(),
                action_type: CommandType::PlayerPauseResume,
                args: vec![],
                user_facing: true,
            },
            CommandDefinition {
                name: "player-next".to_string(),
                action_type: CommandType::PlayerNext,
                args: vec![],
                user_facing: true,
            },
            CommandDefinition {
                name: "player-prev".to_string(),
                action_type: CommandType::PlayerPrev,
                args: vec![],
                user_facing: true,
            },
        ]);

        global_commands
    }

    fn init_global_commands_keys() -> HashMap<KeyCode, String> {
        HashMap::from([
            (KeyCode::Char(':'), "enter_command_mode".to_string()),
            (KeyCode::Char('1'), "view player".to_string()),
            (KeyCode::Char('2'), "view history".to_string()),
            (KeyCode::Char('0'), "view help".to_string()),
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
    fn generate_keys_help_str(key_map: &HashMap<KeyCode, String>) -> String {
        let mut help_str = String::new();

        let min_width = 7;

        for (key_code, command) in key_map {
            help_str += &match key_code {
                KeyCode::Char(' ') => format!("{:min_width$}  {command}", "space"),
                KeyCode::Char(c) => format!("{:min_width$}  {command}", c),
                key_code => format!("{:min_width$?}  {command}", key_code),
            };
            help_str.push('\n');
        }

        let mut help_str: Vec<&str> = help_str.lines().collect();
        help_str.sort();

        help_str.join("\n")
    }

    fn init_history_state_commands() -> CommandsRegistry {
        let mut history_state_commands = CommandsRegistry::new();
        history_state_commands.add_commands([CommandDefinition {
            name: "scroll".to_string(),
            action_type: CommandType::Scroll,
            args: vec![Arg::I16(None)],
            user_facing: true,
        }]);

        history_state_commands
    }

    fn init_history_state_commands_keys() -> HashMap<KeyCode, String> {
        HashMap::from([
            (KeyCode::Char('j'), "scroll +1".to_string()),
            (KeyCode::Char('k'), "scroll -1".to_string()),
        ])
    }

    fn init_help_state_commands() -> CommandsRegistry {
        let mut help_state_commands = CommandsRegistry::new();
        help_state_commands.add_commands([CommandDefinition {
            name: "scroll".to_string(),
            action_type: CommandType::Scroll,
            args: vec![Arg::I16(None)],
            user_facing: true,
        }]);

        help_state_commands
    }

    fn init_help_state_commands_keys() -> HashMap<KeyCode, String> {
        HashMap::from([
            (KeyCode::Char('j'), "scroll +1".to_string()),
            (KeyCode::Char('k'), "scroll -1".to_string()),
        ])
    }
}
