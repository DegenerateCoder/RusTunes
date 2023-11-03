use std::collections::HashMap;

pub struct OptionDefinition {
    pub name: String,
    pub option_type: OptionType,
    pub args: Vec<Arg>,
}

pub enum Arg {
    USIZE(Option<usize>),
    BOOL(Option<bool>),
    I64(Option<i64>),
    U64(Option<u64>),
}

impl Arg {
    fn extract_bool(self) -> Option<bool> {
        match self {
            Arg::BOOL(bool) => bool,
            _ => None,
        }
    }

    fn extract_usize(self) -> Option<usize> {
        match self {
            Arg::USIZE(usize) => usize,
            _ => None,
        }
    }

    fn extract_i64(self) -> Option<i64> {
        match self {
            Arg::I64(i64) => i64,
            _ => None,
        }
    }

    fn extract_u64(self) -> Option<u64> {
        match self {
            Arg::U64(u64) => u64,
            _ => None,
        }
    }
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

    pub fn map_option_to_action(&self, option: &str, args: Vec<&str>) -> Option<Action> {
        let option = self.options.get(option)?;

        let action = &option.action;
        let action_args = &option.args;

        if action_args.len() != args.len() {
            return None;
        }

        let mut processed_args = vec![];
        for (i, arg) in args.iter().enumerate() {
            let processed_arg = match action_args[i] {
                Arg::USIZE(_) => Arg::USIZE(Some(arg.parse().ok()?)),
                Arg::BOOL(_) => Arg::BOOL(Some(arg.parse().ok()?)),
                Arg::I64(_) => Arg::I64(Some(arg.parse().ok()?)),
                Arg::U64(_) => Arg::U64(Some(arg.parse().ok()?)),
            };
            processed_args.push(processed_arg);
        }

        let action_with_args = match action {
            OptionType::SetPipedApiDomainIndex => {
                Action::SetPipedApiDomainIndex(processed_args.pop()?.extract_usize()?)
            }
            OptionType::SetInvidiousApiDomainIndex => {
                Action::SetInvidiousApiDomainIndex(processed_args.pop()?.extract_usize()?)
            }
            OptionType::SetShufflePlaylist => {
                Action::SetShufflePlaylist(processed_args.pop()?.extract_bool()?)
            }
            OptionType::SetMpvBaseVolume => {
                Action::SetMpvBaseVolume(processed_args.pop()?.extract_i64()?)
            }
            OptionType::SetPlayOnlyRecommendations => {
                Action::SetPlayOnlyRecommendations(processed_args.pop()?.extract_bool()?)
            }
            OptionType::SetVideoDurationLimit => {
                Action::SetVideoDurationLimit(processed_args.pop()?.extract_u64()?)
            }
            OptionType::SetDebugLog => Action::SetDebugLog(processed_args.pop()?.extract_bool()?),
        };

        Some(action_with_args)
    }

    pub fn map_option_str_to_action(&self, option: &str) -> Option<Action> {
        let mut option_with_args = option.split('=');

        let option = option_with_args.next()?;
        let args: Vec<&str> = option_with_args
            .next()
            .map(|args| args.split(',').collect())
            .or(Some(Vec::new()))?;

        self.map_option_to_action(option, args)
    }
}

#[derive(Debug)]
pub enum OptionType {
    SetPipedApiDomainIndex,
    SetInvidiousApiDomainIndex,
    SetShufflePlaylist,
    SetMpvBaseVolume,
    SetDebugLog,
    SetPlayOnlyRecommendations,
    SetVideoDurationLimit,
}

#[derive(Debug)]
pub enum Action {
    SetPipedApiDomainIndex(usize),
    SetInvidiousApiDomainIndex(usize),
    SetShufflePlaylist(bool),
    SetMpvBaseVolume(i64),
    SetDebugLog(bool),
    SetPlayOnlyRecommendations(bool),
    SetVideoDurationLimit(u64),
}
