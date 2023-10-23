use crate::music_player::logger::Error;
//use crate::music_player::logger::LogSender;

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
