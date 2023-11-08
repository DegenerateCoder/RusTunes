use crate::music_player::logger::Error;
use crate::music_player::logger::LogSender;
use crate::music_player::tui::commands::commands_registry::Arg;
mod options_registry;
use options_registry::{Action, OptionDefinition, OptionType, OptionsRegistry};

#[derive(serde::Deserialize)]
pub struct MusicPlayerConfig {
    pub piped_api_domains: Vec<String>,
    pub piped_api_domain_index: usize,
    pub shuffle_playlist: bool,
    pub invidious_api_domains: Vec<String>,
    pub invidious_api_domain_index: usize,
    pub mpv_base_volume: i64,
    pub video_duration_limit_s: u64,
    pub debug_log: bool,
    pub play_only_recommendations: bool,
}

impl MusicPlayerConfig {
    pub fn new() -> Result<Self, Error> {
        let config = std::fs::read_to_string("conf.json").unwrap_or_else(|_| {
            let def_conf = std::fs::read_to_string("def_conf.json").unwrap();
            def_conf
        });

        let config: Self = serde_json::from_str(&config)?;

        Ok(config)
    }
}

pub struct MusicPlayerOptions {
    options: OptionsRegistry,
    log_send: LogSender,
}

impl MusicPlayerOptions {
    pub fn new(log_send: LogSender) -> Self {
        Self {
            options: Self::init_options(),
            log_send,
        }
    }

    fn init_options() -> OptionsRegistry {
        let mut options = OptionsRegistry::new();
        options.add_options([
            OptionDefinition {
                name: "--help".to_string(),
                option_type: OptionType::PrintHelp,
                args: vec![],
            },
            OptionDefinition {
                name: "--piped_api_domain_index".to_string(),
                option_type: OptionType::SetPipedApiDomainIndex,
                args: vec![Arg::USIZE(None)],
            },
            OptionDefinition {
                name: "--shuffle_playlist".to_string(),
                option_type: OptionType::SetShufflePlaylist,
                args: vec![Arg::BOOL(None)],
            },
            OptionDefinition {
                name: "--invidious_api_domain_index".to_string(),
                option_type: OptionType::SetInvidiousApiDomainIndex,
                args: vec![Arg::USIZE(None)],
            },
            OptionDefinition {
                name: "--mpv_base_volume".to_string(),
                option_type: OptionType::SetMpvBaseVolume,
                args: vec![Arg::I64(None)],
            },
            OptionDefinition {
                name: "--video_duration_limit_s".to_string(),
                option_type: OptionType::SetVideoDurationLimit,
                args: vec![Arg::U64(None)],
            },
            OptionDefinition {
                name: "--debug_log".to_string(),
                option_type: OptionType::SetDebugLog,
                args: vec![Arg::BOOL(None)],
            },
            OptionDefinition {
                name: "--play_only_recommendations".to_string(),
                option_type: OptionType::SetPlayOnlyRecommendations,
                args: vec![Arg::BOOL(None)],
            },
        ]);

        options
    }

    pub fn process_and_apply_args(
        &self,
        mut config: MusicPlayerConfig,
        args: &[String],
    ) -> Result<MusicPlayerConfig, Error> {
        for arg in args {
            let action = self.options.map_option_str_to_action(arg);
            self.log_send
                .send_log_message(format!("{:?} -> {:?}", arg, action));
            if action.is_none() {
                continue;
            }
            let action = action.unwrap();
            match action {
                Action::PrintHelp => {
                    self.print_help();
                    return Err(Error::PrintHelp);
                }
                Action::SetPipedApiDomainIndex(index) => config.piped_api_domain_index = index,
                Action::SetShufflePlaylist(val) => config.shuffle_playlist = val,
                Action::SetInvidiousApiDomainIndex(index) => {
                    config.invidious_api_domain_index = index
                }
                Action::SetMpvBaseVolume(val) => config.mpv_base_volume = val,
                Action::SetVideoDurationLimit(val) => config.video_duration_limit_s = val,
                Action::SetDebugLog(val) => config.debug_log = val,
                Action::SetPlayOnlyRecommendations(val) => config.play_only_recommendations = val,
            }
        }
        Ok(config)
    }

    pub fn print_help(&self) {
        println!("Usage: rustunes [OPTIONS] URL");
        println!("");
        println!("Options:");

        let options_help = self.options.generate_help_str();
        options_help
            .lines()
            .for_each(|option| println!("  {option}"));
    }
}
