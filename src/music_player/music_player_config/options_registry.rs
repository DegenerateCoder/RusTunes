use crate::music_player::tui::commands::commands_registry::Arg;
use crate::music_player::tui::TuiState;
use action_to_type_macro::ActionToType;
use std::collections::HashMap;

pub struct OptionDefinition {
    pub name: String,
    pub option_type: OptionType,
    pub args: Vec<Arg>,
}

pub struct OptionsRegistry {
    options: HashMap<String, OptionEntry>,
}

pub struct OptionEntry {
    action: OptionType,
    args: Vec<Arg>,
}

impl OptionsRegistry {
    pub fn new() -> Self {
        OptionsRegistry {
            options: HashMap::new(),
        }
    }

    pub fn add_option(&mut self, option: OptionDefinition) {
        self.options.insert(
            option.name,
            OptionEntry {
                action: option.option_type,
                args: option.args,
            },
        );
    }

    pub fn add_options<const N: usize>(&mut self, options: [OptionDefinition; N]) {
        for option in options {
            self.add_option(option);
        }
    }

    pub fn map_option_to_action(&self, option: &str, args: Vec<&str>) -> Option<OptionAction> {
        let option = self.options.get(option)?;

        let action = &option.action;
        let action_args = &option.args;

        if action_args.len() != args.len() {
            return None;
        }

        let mut processed_args = vec![];
        for (i, arg) in args.iter().enumerate() {
            let processed_arg = match action_args[i] {
                Arg::TUISTATE(_) => Arg::TUISTATE(match *arg {
                    "player" => Some(TuiState::Player),
                    "history" => Some(TuiState::History),
                    _ => None,
                }),
                Arg::BOOL(_) => Arg::BOOL(Some(arg.parse().ok()?)),
                Arg::I16(_) => Arg::I16(Some(arg.parse().ok()?)),
                Arg::I64(_) => Arg::I64(Some(arg.parse().ok()?)),
                Arg::U64(_) => Arg::U64(Some(arg.parse().ok()?)),
                Arg::USIZE(_) => Arg::USIZE(Some(arg.parse().ok()?)),
            };
            processed_args.push(processed_arg);
        }

        let action_with_args = action.map_type_to_action(processed_args)?;

        Some(action_with_args)
    }

    pub fn map_option_str_to_action(&self, option: &str) -> Option<OptionAction> {
        let mut option_with_args = option.split('=');

        let option = option_with_args.next()?;
        let args: Vec<&str> = option_with_args
            .next()
            .map(|args| args.split(',').collect())
            .or(Some(Vec::new()))?;

        self.map_option_to_action(option, args)
    }

    pub fn generate_help_str(&self) -> String {
        let mut help_str = String::new();

        for (option, option_entry) in &self.options {
            let mut str = format!("{option}");

            if !option_entry.args.is_empty() {
                str += "=<";
            }
            option_entry.args.iter().for_each(|arg| {
                str += &format!("{},", arg.to_type_str());
            });

            if !option_entry.args.is_empty() {
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

    pub fn get_options_names(&self) -> Vec<&String> {
        self.options.keys().collect()
    }
}

#[derive(Debug, PartialEq, Eq, ActionToType)]
pub enum OptionAction {
    PrintHelp,
    SetPipedApiDomainIndex(usize),
    SetInvidiousApiDomainIndex(usize),
    SetShufflePlaylist(bool),
    SetMpvBaseVolume(i64),
    SetDebugLog(bool),
    SetPlayOnlyRecommendations(bool),
    SetVideoDurationLimit(u64),
    RankPipedApiDomains,
    RankInvidiousApiDomains,
    FetchPipedApiDomains,
    FetchInvidiousApiDomains,
    OverwriteConfig,
}
