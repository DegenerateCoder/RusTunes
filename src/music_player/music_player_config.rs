use crate::music_player::logger::Error;
use crate::music_player::tui::commands::commands_registry::Arg;
pub mod options_registry;
use crate::music_player::music_player_core::music_source::Source;
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

    pub fn apply_simple_actions(&mut self, actions: Vec<Action>) -> Vec<Action> {
        let config = self;
        let mut complex_actions = Vec::new();

        for action in actions {
            match action {
                Action::SetPipedApiDomainIndex(index) => config.piped_api_domain_index = index,
                Action::SetShufflePlaylist(val) => config.shuffle_playlist = val,
                Action::SetInvidiousApiDomainIndex(index) => {
                    config.invidious_api_domain_index = index
                }
                Action::SetMpvBaseVolume(val) => config.mpv_base_volume = val,
                Action::SetVideoDurationLimit(val) => config.video_duration_limit_s = val,
                Action::SetDebugLog(val) => config.debug_log = val,
                Action::SetPlayOnlyRecommendations(val) => config.play_only_recommendations = val,
                Action::PrintHelp => (),
                Action::RankPipedApiDomains => complex_actions.push(Action::RankPipedApiDomains),
                Action::RankInvidiousApiDomains => {
                    complex_actions.push(Action::RankInvidiousApiDomains)
                }
                Action::FetchPipedApiDomains => complex_actions.push(Action::FetchPipedApiDomains),
                Action::FetchInvidiousApiDomains => {
                    complex_actions.push(Action::FetchInvidiousApiDomains)
                }
            }
        }

        if !config.debug_log {
            log::set_max_level(log::LevelFilter::Off);
        }

        complex_actions
    }

    pub fn apply_complex_actions(&mut self, actions: Vec<Action>) -> Result<(), Error> {
        let config = self;
        let mut rank_piped_api_domains = false;
        let mut rank_invidious_api_domains = false;
        let mut fetch_piped_api_domains = false;
        let mut fetch_invidious_api_domains = false;

        for action in actions {
            match action {
                Action::RankPipedApiDomains => rank_piped_api_domains = true,
                Action::RankInvidiousApiDomains => rank_invidious_api_domains = true,
                Action::FetchPipedApiDomains => fetch_piped_api_domains = true,
                Action::FetchInvidiousApiDomains => fetch_invidious_api_domains = true,
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

    pub fn preprocess_args(&self, args: &[String]) -> Result<Vec<Action>, Error> {
        let mut actions = Vec::new();

        for arg in args {
            let action = self.options.map_option_str_to_action(arg);

            log::info!("{:?} -> {:?}", arg, action);

            if let Some(action) = action {
                actions.push(action);
            }
        }

        Ok(actions)
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

    pub fn rank_piped_api_domains(config: &mut MusicPlayerConfig) -> Result<(), Error> {
        println!("Ranking Piped API domains: ");
        log::info!("MusicPlayerOptions::rank_piped_api_domains");

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
                        log::info!("\t{piped_api_domain}: {elapsed}ms");

                        ranking_queue.push((piped_api_domain, elapsed)).unwrap();
                    } else {
                        println!("\t{piped_api_domain}: ERROR");
                        log::info!("\t{piped_api_domain}: {:?}", response.err());
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
                        log::info!("\t{invidious_api_domain}: {elapsed}ms");

                        ranking_queue.push((invidious_api_domain, elapsed)).unwrap();
                    } else {
                        println!("\t{invidious_api_domain}: ERROR");
                        log::info!("\t{invidious_api_domain}: {:?}", response.err());
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
        let mut piped_api_domains = Vec::new();

        let request_url = "https://piped-instances.kavin.rocks/";
        let reqwest_client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .build()
            .unwrap();
        let request = reqwest_client.get(request_url).build()?;
        let response: serde_json::Value = reqwest_client.execute(request)?.json()?;

        let instances = response
            .as_array()
            .ok_or_else(|| Error::OtherError(format!("{:?}", response.to_string())))?;

        for instance in instances {
            let api_url = instance
                .get("api_url")
                .ok_or_else(|| Error::OtherError(format!("{:?}", response.to_string())))?;
            let api_url = api_url.as_str().unwrap();

            println!("\t{}: {api_url}", piped_api_domains.len());
            log::info!("\t{}: {api_url}", piped_api_domains.len());

            piped_api_domains.push(api_url.to_string());
        }

        log::info!(
            "MusicPlayerOptions::fetch_piped_api_domains -> {:?}",
            piped_api_domains
        );
        Ok(piped_api_domains)
    }

    pub fn fetch_invidious_api_domains() -> Result<Vec<String>, Error> {
        println!("Fetching Invidious API domains: ");
        log::info!("MusicPlayerOptions::fetch_invidious_api_domains");
        let mut invidious_api_domains = Vec::new();

        let request_url = "https://api.invidious.io/instances.json?pretty=0&sort_by=type,health";
        let reqwest_client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .build()
            .unwrap();
        let request = reqwest_client.get(request_url).build()?;
        let response: serde_json::Value = reqwest_client.execute(request)?.json()?;

        let instances = response
            .as_array()
            .ok_or_else(|| Error::OtherError(format!("{:?}", response.to_string())))?;

        for instance in instances {
            let instance_data = instance
                .get(1)
                .ok_or_else(|| Error::OtherError(format!("{:?}", instance.to_string())))?;
            let api = instance_data
                .get("api")
                .ok_or_else(|| Error::OtherError(format!("{:?}", instance.to_string())))?
                .as_bool();
            if let Some(api) = api {
                if !api {
                    continue;
                }
            } else {
                continue;
            }
            let api_url = instance_data
                .get("uri")
                .ok_or_else(|| Error::OtherError(format!("{:?}", instance.to_string())))?;
            let api_url = api_url.as_str().unwrap();

            println!("\t{}: {api_url}", invidious_api_domains.len());
            log::info!("\t{}: {api_url}", invidious_api_domains.len());

            invidious_api_domains.push(api_url.to_string());
        }

        log::info!(
            "MusicPlayerOptions::fetch_invidious_api_domains -> {:?}",
            invidious_api_domains
        );
        Ok(invidious_api_domains)
    }
}
