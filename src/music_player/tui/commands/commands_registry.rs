use crate::music_player::tui::TuiState;
use std::collections::hash_map::HashMap;

pub struct CommandDefinition {
    pub name: String,
    pub action_type: CommandType,
    pub args: Vec<Arg>,
    pub user_facing: bool,
}

pub enum Arg {
    TuiState(Option<TuiState>),
    USIZE(Option<usize>),
    BOOL(Option<bool>),
    I64(Option<i64>),
    I16(Option<i16>),
    U64(Option<u64>),
}

impl Arg {
    pub fn extract_tui_state(self) -> Option<TuiState> {
        match self {
            Arg::TuiState(tui_state) => tui_state,
            _ => None,
        }
    }

    pub fn extract_bool(self) -> Option<bool> {
        match self {
            Arg::BOOL(bool) => bool,
            _ => None,
        }
    }

    pub fn extract_usize(self) -> Option<usize> {
        match self {
            Arg::USIZE(usize) => usize,
            _ => None,
        }
    }

    pub fn extract_i16(self) -> Option<i16> {
        match self {
            Arg::I16(i16) => i16,
            _ => None,
        }
    }

    pub fn extract_i64(self) -> Option<i64> {
        match self {
            Arg::I64(i64) => i64,
            _ => None,
        }
    }

    pub fn extract_u64(self) -> Option<u64> {
        match self {
            Arg::U64(u64) => u64,
            _ => None,
        }
    }

    pub fn to_type_str(&self) -> &'static str {
        match self {
            Arg::TuiState(_) => "TuiState",
            Arg::USIZE(_) => "usize",
            Arg::BOOL(_) => "bool",
            Arg::I16(_) => "i16",
            Arg::I64(_) => "i64",
            Arg::U64(_) => "u64",
        }
    }
}

pub struct CommandEntry {
    action: CommandType,
    args: Vec<Arg>,
    user_facing: bool,
}

pub struct CommandsRegistry {
    commands: HashMap<String, CommandEntry>,
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
                user_facing: command.user_facing,
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
                    "help" => Some(TuiState::Help),
                    _ => None,
                }),
                Arg::I64(_) => Arg::I64(Some(arg.parse().ok()?)),
                Arg::I16(_) => Arg::I16(Some(arg.parse().ok()?)),
                Arg::USIZE(_) => Arg::USIZE(Some(arg.parse().ok()?)),
                Arg::U64(_) => Arg::U64(Some(arg.parse().ok()?)),
                Arg::BOOL(_) => Arg::BOOL(Some(arg.parse().ok()?)),
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

    pub fn generate_help_str(&self) -> String {
        let mut help_str = String::new();

        for (command, command_entry) in &self.commands {
            if !command_entry.user_facing {
                continue;
            }

            let mut str = format!("{command}");

            if !command_entry.args.is_empty() {
                str += "=<";
            }
            command_entry.args.iter().for_each(|arg| {
                str += &format!("{},", arg.to_type_str());
            });

            if !command_entry.args.is_empty() {
                str.pop();
                str += ">";
            }

            help_str += &str;
            help_str.push('\n');
        }

        let mut help_str: Vec<&str> = help_str.lines().collect();
        help_str.sort();

        help_str.join("\n")
    }

    pub fn get_commands_names(&self) -> Vec<&String> {
        self.commands
            .keys()
            .filter(|key| self.commands.get(*key).unwrap().user_facing)
            .collect()
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
