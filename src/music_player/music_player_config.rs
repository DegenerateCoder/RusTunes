use crate::music_player::tui::commands::commands_registry::Arg;
use crate::music_player::Error;
pub mod options_registry;
use crate::music_player::music_player_core::music_source::Source;
use crate::utils;
use options_registry::{OptionAction, OptionDefinition, OptionType, OptionsRegistry};

#[derive(serde::Deserialize, serde::Serialize)]
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
        let user_config = std::fs::read_to_string("conf.json");
        let config: Self = {
            if user_config.is_ok() {
                serde_json::from_str(user_config.unwrap().as_str())?
            } else {
                println!("Using default config");
                log::info!("Using default config");
                let def_conf = MusicPlayerConfig::get_def_conf();
                serde_json::from_str(def_conf)?
            }
        };

        Ok(config)
    }

    fn get_def_conf() -> &'static str {
        r#"
        {
          "piped_api_domains": [
            "https://piped-api.garudalinux.org"
          ],
          "piped_api_domain_index": 0,
          "invidious_api_domains": [
            "https://invidious.garudalinux.org"
          ],
          "invidious_api_domain_index": 0,
          "mpv_base_volume": 100,
          "video_duration_limit_s": 600,
          "shuffle_playlist": true,
          "play_only_recommendations": false,
          "debug_log": false
        }
        "#
    }

    pub fn apply_simple_actions(&mut self, actions: Vec<OptionAction>) -> Vec<OptionAction> {
        let config = self;
        let mut complex_actions = Vec::new();

        for action in actions {
            match action {
                OptionAction::SetPipedApiDomainIndex(index) => {
                    config.piped_api_domain_index = index
                }
                OptionAction::SetShufflePlaylist(val) => config.shuffle_playlist = val,
                OptionAction::SetInvidiousApiDomainIndex(index) => {
                    config.invidious_api_domain_index = index
                }
                OptionAction::SetMpvBaseVolume(val) => config.mpv_base_volume = val,
                OptionAction::SetVideoDurationLimit(val) => config.video_duration_limit_s = val,
                OptionAction::SetDebugLog(val) => config.debug_log = val,
                OptionAction::SetPlayOnlyRecommendations(val) => {
                    config.play_only_recommendations = val
                }
                OptionAction::PrintHelp => (),
                OptionAction::OverwriteConfig => (),
                OptionAction::RankPipedApiDomains => {
                    complex_actions.push(OptionAction::RankPipedApiDomains)
                }
                OptionAction::RankInvidiousApiDomains => {
                    complex_actions.push(OptionAction::RankInvidiousApiDomains)
                }
                OptionAction::FetchPipedApiDomains => {
                    complex_actions.push(OptionAction::FetchPipedApiDomains)
                }
                OptionAction::FetchInvidiousApiDomains => {
                    complex_actions.push(OptionAction::FetchInvidiousApiDomains)
                }
            }
        }

        if !config.debug_log {
            log::set_max_level(log::LevelFilter::Off);
        }

        complex_actions
    }

    pub fn apply_complex_actions(&mut self, actions: Vec<OptionAction>) -> Result<(), Error> {
        let config = self;
        let mut rank_piped_api_domains = false;
        let mut rank_invidious_api_domains = false;
        let mut fetch_piped_api_domains = false;
        let mut fetch_invidious_api_domains = false;

        for action in actions {
            match action {
                OptionAction::RankPipedApiDomains => rank_piped_api_domains = true,
                OptionAction::RankInvidiousApiDomains => rank_invidious_api_domains = true,
                OptionAction::FetchPipedApiDomains => fetch_piped_api_domains = true,
                OptionAction::FetchInvidiousApiDomains => fetch_invidious_api_domains = true,
                _ => (),
            }
        }

        if fetch_piped_api_domains {
            let piped_api_domains = MusicPlayerOptions::fetch_piped_api_domains()?;
            config.piped_api_domains = piped_api_domains;
            config.piped_api_domain_index = 0;
        }
        if fetch_invidious_api_domains {
            let invidious_api_domains = MusicPlayerOptions::fetch_invidious_api_domains()?;
            config.invidious_api_domains = invidious_api_domains;
            config.invidious_api_domain_index = 0;
        }
        if rank_piped_api_domains {
            MusicPlayerOptions::rank_piped_api_domains(config)?;
        }
        if rank_invidious_api_domains {
            MusicPlayerOptions::rank_invidious_api_domains(config)?;
        }

        Ok(())
    }

    pub fn overwrite_config(&self, override_conf: bool) -> Result<(), Error> {
        if !override_conf {
            return Ok(());
        }
        let conf_json = serde_json::to_string_pretty(&self)?;
        println!("Overwriting config: \n{conf_json}");
        log::info!("MusicPlayerConfig::overwrite_config: \n{conf_json}");
        std::fs::write("conf.json", conf_json)?;

        Ok(())
    }
}

pub struct MusicPlayerOptions {
    options: OptionsRegistry,
}

impl MusicPlayerOptions {
    pub fn new() -> Self {
        Self {
            options: Self::init_options(),
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
            OptionDefinition {
                name: "--fetch_piped_api_domains".to_string(),
                option_type: OptionType::FetchPipedApiDomains,
                args: vec![],
            },
            OptionDefinition {
                name: "--fetch_invidious_api_domains".to_string(),
                option_type: OptionType::FetchInvidiousApiDomains,
                args: vec![],
            },
            OptionDefinition {
                name: "--overwrite_config".to_string(),
                option_type: OptionType::OverwriteConfig,
                args: vec![],
            },
        ]);

        options
    }

    pub fn extract_user_input_url(mut args: Vec<String>) -> (Vec<String>, Option<String>) {
        let user_input: Option<String> = {
            if args.len() > 1 && Source::is_valid_source_path(args.last().unwrap()) {
                Some(args.pop().unwrap())
            } else {
                None
            }
        };

        (args, user_input)
    }

    pub fn preprocess_args(&self, args: &[String]) -> Result<Vec<OptionAction>, Error> {
        let mut actions = Vec::new();

        for arg in args {
            let action = self.options.map_option_str_to_action(arg);

            log::info!("{:?} -> {:?}", arg, action);

            if let Some(action) = action {
                actions.push(action);
            } else {
                let invalid_arg = arg.split('=').next().unwrap();
                let suggestion = self.find_suggestion(invalid_arg);
                return Err(Error::InvalidOption(format!(
                    "rustunes: '{invalid_arg}' is not a valid rustunes option.\nDid you mean: {suggestion}"
                )));
            }
        }

        Ok(actions)
    }

    pub fn find_suggestion(&self, invalid_arg: &str) -> &str {
        let a = invalid_arg;
        let mut min_lev_distances = u8::MAX;
        let mut suggestion = "";

        let options_names = self.options.get_options_names();
        for b in options_names {
            let lev_dist = lev(a, b);
            if lev_dist < min_lev_distances {
                min_lev_distances = lev_dist;
                suggestion = b;
            }
        }

        suggestion
    }

    pub fn print_help(&self) {
        println!("Usage: rustunes [OPTIONS] URL");
        println!("       rustunes --overwrite_config [OPTIONS]");
        println!("");
        println!("Options:");

        let options_help = self.options.generate_help_str();
        options_help
            .lines()
            .for_each(|option| println!("  {option}"));
    }

    pub fn rank_piped_api_domains(config: &mut MusicPlayerConfig) -> Result<(), Error> {
        println!("Ranking Piped API domains: ");
        log::info!("MusicPlayerOptions::rank_piped_api_domains");

        let ranking_queue = crossbeam::queue::ArrayQueue::new(config.piped_api_domains.len());

        let piped_api_domains = &config.piped_api_domains;
        crossbeam::scope(|scope| {
            for piped_api_domain in piped_api_domains {
                scope.spawn(|_| {
                    let piped_api_domain = piped_api_domain.clone();
                    let request_url = format!("{}/streams/{}", piped_api_domain, "dQw4w9WgXcQ");

                    let elapsed = utils::measure_reqwest_get_duration(&request_url);

                    if elapsed.is_ok() {
                        let elapsed = elapsed.unwrap().as_millis();

                        println!("\t{piped_api_domain}: {elapsed}ms");
                        log::info!("\t{piped_api_domain}: {elapsed}ms");

                        ranking_queue.push((piped_api_domain, elapsed)).unwrap();
                    } else {
                        println!("\t{piped_api_domain}: ERROR");
                        log::info!("\t{piped_api_domain}: {:?}", elapsed.err());
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

        if ranking.is_empty() {
            return Err(Error::AllPipedApiDomainsDown(
                "All provided piped api domains are down".to_owned(),
            ));
        }
        config.piped_api_domains.clear();
        for piped_api_domain in ranking {
            config.piped_api_domains.push(piped_api_domain.0);
        }
        config.piped_api_domain_index = 0;

        println!("Piped API domain set to: {}", config.piped_api_domains[0]);
        log::info!("Piped API domain set to: {}", config.piped_api_domains[0]);

        Ok(())
    }

    pub fn rank_invidious_api_domains(config: &mut MusicPlayerConfig) -> Result<(), Error> {
        println!("Ranking Invidious API domains: ");
        log::info!("MusicPlayerOptions::rank_invidious_api_domains");

        let ranking_queue = crossbeam::queue::ArrayQueue::new(config.invidious_api_domains.len());

        let invidious_api_domains = &config.invidious_api_domains;
        crossbeam::scope(|scope| {
            for invidious_api_domain in invidious_api_domains {
                scope.spawn(|_| {
                    let invidious_api_domain = invidious_api_domain.clone();
                    let request_url =
                        format!("{}/api/v1/videos/{}", invidious_api_domain, "dQw4w9WgXcQ");

                    let elapsed = utils::measure_reqwest_get_duration(&request_url);

                    if elapsed.is_ok() {
                        let elapsed = elapsed.unwrap().as_millis();

                        println!("\t{invidious_api_domain}: {elapsed}ms");
                        log::info!("\t{invidious_api_domain}: {elapsed}ms");

                        ranking_queue.push((invidious_api_domain, elapsed)).unwrap();
                    } else {
                        println!("\t{invidious_api_domain}: ERROR");
                        log::info!("\t{invidious_api_domain}: {:?}", elapsed.err());
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

        if ranking.is_empty() {
            return Err(Error::AllInvidiousApiDomainsDown(
                "All provided invidious api domains are down".to_owned(),
            ));
        }
        config.invidious_api_domains.clear();
        for invidious_api_domain in ranking {
            config.invidious_api_domains.push(invidious_api_domain.0);
        }
        config.invidious_api_domain_index = 0;

        println!(
            "Invidious API domain set to: {}",
            config.invidious_api_domains[0]
        );
        log::info!(
            "Invidious API domain set to: {}",
            config.invidious_api_domains[0]
        );

        Ok(())
    }

    pub fn fetch_piped_api_domains() -> Result<Vec<String>, Error> {
        println!("Fetching Piped API domains: ");
        log::info!("MusicPlayerOptions::fetch_piped_api_domains");

        let piped_api_domains = utils::fetch_piped_api_domains()?;

        piped_api_domains
            .iter()
            .enumerate()
            .for_each(|(i, api_url)| println!("\t{i}: {api_url}"));

        log::info!(
            "MusicPlayerOptions::fetch_piped_api_domains -> {:?}",
            piped_api_domains
        );

        Ok(piped_api_domains)
    }

    pub fn fetch_invidious_api_domains() -> Result<Vec<String>, Error> {
        println!("Fetching Invidious API domains: ");
        log::info!("MusicPlayerOptions::fetch_invidious_api_domains");

        let invidious_api_domains = utils::fetch_invidious_api_domains()?;

        invidious_api_domains
            .iter()
            .enumerate()
            .for_each(|(i, api_url)| println!("\t{i}: {api_url}"));

        log::info!(
            "MusicPlayerOptions::fetch_invidious_api_domains -> {:?}",
            invidious_api_domains
        );

        Ok(invidious_api_domains)
    }
}

// https://en.wikipedia.org/wiki/Levenshtein_distance
pub fn lev(a: &str, b: &str) -> u8 {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut v0 = vec![0usize; a.len() + 1];
    let mut v1 = vec![0usize; a.len() + 1];

    for (i, v) in v0.iter_mut().enumerate() {
        *v = i;
    }

    for y in 1..=b.len() {
        v1[0] = v0[0] + 1;

        for x in 1..v0.len() {
            if b[y - 1] == a[x - 1] {
                v1[x] = v0[x - 1] + 0;
            } else {
                let mut lev = [0usize; 3];
                lev[0] = v0[x];
                lev[1] = v0[x - 1];
                lev[2] = v1[x - 1];
                let min_lev = lev.iter().min().unwrap().to_owned();
                v1[x] = 1 + min_lev;
            }
        }

        v0 = v1;
        v1 = vec![0usize; a.len() + 1];
    }

    v0.last().unwrap().to_owned().try_into().unwrap()
}
