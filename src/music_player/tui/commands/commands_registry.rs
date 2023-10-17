use crate::music_player::tui::TuiState;
use std::collections::hash_map::HashMap;

pub struct CommandDefinition {
    pub name: String,
    pub action_type: CommandType,
    pub args: Vec<Arg>,
}

pub enum Arg {
    TuiState(Option<TuiState>),
    I64(Option<i64>),
    I16(Option<i16>),
}

impl Arg {
    fn extract_tui_state(self) -> Option<TuiState> {
        match self {
            Arg::TuiState(tui_state) => tui_state,
            _ => None,
        }
    }

    fn extract_i64(self) -> Option<i64> {
        match self {
            Arg::I64(i64) => i64,
            _ => None,
        }
    }

    fn extract_i16(self) -> Option<i16> {
        match self {
            Arg::I16(i16) => i16,
            _ => None,
        }
    }
}

pub struct CommandsRegistry {
    commands: HashMap<String, CommandEntry>,
}

pub struct CommandEntry {
    action: CommandType,
    args: Vec<Arg>,
}

impl CommandsRegistry {
    pub fn new() -> Self {
        CommandsRegistry {
            commands: HashMap::new(),
        }
    }
    pub fn add_command(&mut self, command: CommandDefinition) {
        self.commands.insert(
            command.name,
            CommandEntry {
                action: command.action_type,
                args: command.args,
            },
        );
    }
    pub fn add_commands<const N: usize>(&mut self, commands: [CommandDefinition; N]) {
        for command in commands {
            self.add_command(command);
        }
    }

    pub fn map_command_to_action(&self, command: &str, args: Vec<&str>) -> Option<Action> {
        let command = self.commands.get(command)?;

        let action = &command.action;
        let action_args = &command.args;

        if action_args.len() != args.len() {
            return None;
        }

        let mut processed_args = vec![];
        for (i, arg) in args.iter().enumerate() {
            let processed_arg = match action_args[i] {
                Arg::TuiState(_) => Arg::TuiState(match *arg {
                    "player" => Some(TuiState::Player),
                    "history" => Some(TuiState::History),
                    _ => None,
                }),
                Arg::I64(_) => Arg::I64(Some(arg.parse().ok()?)),
                Arg::I16(_) => Arg::I16(Some(arg.parse().ok()?)),
            };
            processed_args.push(processed_arg);
        }

        let action_with_args = match action {
            CommandType::EnterCommandMode => Action::EnterCommandMode,
            CommandType::View => Action::View(processed_args.pop()?.extract_tui_state()?),
            CommandType::Quit => Action::Quit,
            CommandType::PlayerPauseResume => Action::PlayerPauseResume,
            CommandType::PlayerNext => Action::PlayerNext,
            CommandType::PlayerPrev => Action::PlayerPrev,
            CommandType::Vol => Action::Vol(processed_args.pop()?.extract_i64()?),
            CommandType::Scroll => Action::Scroll(processed_args.pop()?.extract_i16()?),
        };

        Some(action_with_args)
    }

    pub fn map_command_str_to_action(&self, command: &str) -> Option<Action> {
        let mut command_with_args = command.split(' ');

        let command = command_with_args.next()?;
        let args: Vec<&str> = command_with_args.collect();

        self.map_command_to_action(command, args)
    }
}

#[derive(Debug)]
pub enum CommandType {
    EnterCommandMode,
    View,
    Quit,
    PlayerPauseResume,
    PlayerNext,
    PlayerPrev,
    Vol,
    Scroll,
}

#[derive(Debug)]
pub enum Action {
    EnterCommandMode,
    View(TuiState),
    Quit,
    PlayerPauseResume,
    PlayerNext,
    PlayerPrev,
    Vol(i64),
    Scroll(i16),
}
