use crate::music_player::logger::Error;
use crate::music_player::logger::{LogSender, Logger};
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
            OptionDefinition {
                name: "--rank_piped_api_domains".to_string(),
                option_type: OptionType::RankPipedApiDomains,
                args: vec![],
            },
            OptionDefinition {
                name: "--rank_invidious_api_domains".to_string(),
                option_type: OptionType::RankInvidiousApiDomains,
                args: vec![],
            },
        ]);

        options
    }

    pub fn process_and_apply_args(
        &self,
        mut config: MusicPlayerConfig,
        args: &[String],
    ) -> Result<MusicPlayerConfig, Error> {
        let mut rank_piped_api_domains = false;
        let mut rank_invidious_api_domains = false;
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
                Action::RankPipedApiDomains => rank_piped_api_domains = true,
                Action::RankInvidiousApiDomains => rank_invidious_api_domains = true,
            }
        }

        if rank_piped_api_domains {
            Self::rank_piped_api_domains(&mut config);
        }
        if rank_invidious_api_domains {
            Self::rank_invidious_api_domains(&mut config);
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

    pub fn rank_piped_api_domains(config: &mut MusicPlayerConfig) {
        let logging_enabled = config.debug_log;
        println!("Ranking Piped API domains: ");
        Logger::conditional_log(
            &format!("MusicPlayerOptions::rank_piped_api_domains"),
            logging_enabled,
        )
        .unwrap();

        let ranking_queue = crossbeam::queue::ArrayQueue::new(config.piped_api_domains.len());

        let piped_api_domains = &config.piped_api_domains;
        crossbeam::scope(|scope| {
            for piped_api_domain in piped_api_domains {
                scope.spawn(|_| {
                    let piped_api_domain = piped_api_domain.clone();
                    let reqwest_client = reqwest::blocking::Client::builder()
                        .timeout(std::time::Duration::from_secs(5))
                        .build()
                        .unwrap();

                    let request_url = format!("{}/streams/{}", piped_api_domain, "dQw4w9WgXcQ");
                    let request = reqwest_client.get(request_url).build().unwrap();

                    let start = std::time::SystemTime::now();
                    let response = reqwest_client.execute(request);
                    if response.is_ok() {
                        let elapsed = start.elapsed().unwrap().as_millis();
                        println!("\t{piped_api_domain}: {elapsed}ms");
                        Logger::conditional_log(
                            &format!("\t{piped_api_domain}: {elapsed}ms"),
                            logging_enabled,
                        )
                        .unwrap();

                        ranking_queue.push((piped_api_domain, elapsed)).unwrap();
                    } else {
                        println!("\t{piped_api_domain}: ERROR");
                        Logger::conditional_log(
                            &format!("\t{piped_api_domain}: {:?}", response.err()),
                            logging_enabled,
                        )
                        .unwrap();
                    }
                });
            }
        })
        .unwrap();

        let mut ranking = Vec::new();
        for (piped_api_domain, elapsed) in ranking_queue {
            ranking.push((piped_api_domain, elapsed));
        }
        ranking.sort_by_key(|(_, elapsed)| elapsed.to_owned());

        config.piped_api_domains.clear();
        for piped_api_domain in ranking {
            config.piped_api_domains.push(piped_api_domain.0);
        }
        config.piped_api_domain_index = 0;

        println!("Piped API domain set to: {}", config.piped_api_domains[0]);
    }

    pub fn rank_invidious_api_domains(config: &mut MusicPlayerConfig) {
        let logging_enabled = config.debug_log;
        println!("Ranking Invidious API domains: ");
        Logger::conditional_log(
            &format!("MusicPlayerOptions::rank_invidious_api_domains"),
            logging_enabled,
        )
        .unwrap();

        let ranking_queue = crossbeam::queue::ArrayQueue::new(config.invidious_api_domains.len());

        let invidious_api_domains = &config.invidious_api_domains;
        crossbeam::scope(|scope| {
            for invidious_api_domain in invidious_api_domains {
                scope.spawn(|_| {
                    let invidious_api_domain = invidious_api_domain.clone();
                    let reqwest_client = reqwest::blocking::Client::builder()
                        .timeout(std::time::Duration::from_secs(5))
                        .build()
                        .unwrap();

                    let request_url =
                        format!("{}/api/v1/videos/{}", invidious_api_domain, "dQw4w9WgXcQ");
                    let request = reqwest_client.get(request_url).build().unwrap();

                    let start = std::time::SystemTime::now();
                    let response = reqwest_client.execute(request);
                    if response.is_ok() {
                        let elapsed = start.elapsed().unwrap().as_millis();
                        println!("\t{invidious_api_domain}: {elapsed}ms");
                        Logger::conditional_log(
                            &format!("\t{invidious_api_domain}: {elapsed}ms"),
                            logging_enabled,
                        )
                        .unwrap();

                        ranking_queue.push((invidious_api_domain, elapsed)).unwrap();
                    } else {
                        println!("\t{invidious_api_domain}: ERROR");
                        Logger::conditional_log(
                            &format!("\t{invidious_api_domain}: {:?}", response.err()),
                            logging_enabled,
                        )
                        .unwrap();
                    }
                });
            }
        })
        .unwrap();

        let mut ranking = Vec::new();
        for (invidious_api_domain, elapsed) in ranking_queue {
            ranking.push((invidious_api_domain, elapsed));
        }
        ranking.sort_by_key(|(_, elapsed)| elapsed.to_owned());

        config.invidious_api_domains.clear();
        for invidious_api_domain in ranking {
            config.invidious_api_domains.push(invidious_api_domain.0);
        }
        config.invidious_api_domain_index = 0;

        println!(
            "Invidious API domain set to: {}",
            config.invidious_api_domains[0]
        );
        Logger::conditional_log(
            &format!(
                "Invidious API domain set to: {}",
                config.invidious_api_domains[0]
            ),
            logging_enabled,
        )
        .unwrap();
    }
}
