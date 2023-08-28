fn main() {
    let args: Vec<String> = std::env::args().collect();
    let user_input: Option<&String> = args.get(1);
    let music_player = MusicPlayer::new();
    if let Some(user_input) = user_input {
        music_player.play(user_input);
    } else {
        println!("No input given");
    }
}

struct MusicPlayer {
    music_to_play: String,
    playlist_to_play: String,
    play_video: bool,
    piped_api_domains: Vec<String>,
    piped_api_domain_index: usize,
    invidious_api_domains: Vec<String>,
    invidious_api_domain_index: usize,
    played_video_ids: Vec<String>,
    duration_limit: u64,
}

impl MusicPlayer {
    fn new() -> MusicPlayer {
        MusicPlayer {
            music_to_play: "".to_string(),
            playlist_to_play: "".to_string(),
            play_video: false,
            piped_api_domains: vec!["https://piped-api.garudalinux.org".to_string()],
            piped_api_domain_index: 0,
            invidious_api_domains: vec!["https://invidious.garudalinux.org".to_string()],
            invidious_api_domain_index: 0,
            played_video_ids: Vec::new(),
            duration_limit: 600,
        }
    }
    fn play(mut self, user_input: &str) {
        if user_input.contains("list=") {
            self.playlist_to_play = user_input.to_string();
            self.play_playlist()
        } else if self.play_video {
            self.music_to_play = user_input.to_string();
            self.play_audio_and_video()
        } else {
            self.music_to_play = user_input.to_string();
            self.play_audio();
        }
    }
    fn play_audio(&mut self) {
        self.music_to_play = self.url_into_video_id(&self.music_to_play);
        self.played_video_ids.push(self.music_to_play.clone());
        self.get_audio_url();
        loop {
            self.play_music_mpv();
        }
    }

    fn play_playlist(&self) {
        unimplemented!("playlist playback");
    }

    fn play_audio_and_video(&self) {
        unimplemented!("audio and video playback");
    }

    fn play_music_mpv(&mut self) {
        let mpv = libmpv::Mpv::new().unwrap();
        mpv.set_property("volume", 100).unwrap();
        mpv.set_property("vo", "null").unwrap();

        let mut ev_ctx = mpv.create_event_context();
        ev_ctx.disable_deprecated_events().unwrap();
        /*
        ev_ctx
            .observe_property("volume", libmpv::Format::Int64, 0)
            .unwrap();
        ev_ctx
            .observe_property("demuxer-cache-state", libmpv::Format::Node, 0)
            .unwrap();
        */
        crossbeam::scope(|scope| {
            scope.spawn(|_| {
                mpv.playlist_load_files(&[(
                    &self.music_to_play,
                    libmpv::FileState::AppendPlay,
                    None,
                )])
                .unwrap();
                self.prepare_next_to_play();

                // Trigger `Event::EndFile`.
                //mpv.playlist_next_force().unwrap();
            });
            scope.spawn(move |_| loop {
                let ev = ev_ctx.wait_event(600.).unwrap_or(Err(libmpv::Error::Null));

                match ev {
                    Ok(libmpv::events::Event::EndFile(r)) => {
                        println!("Exiting! Reason: {:?}", r);
                        break;
                    }

                    Ok(libmpv::events::Event::PropertyChange {
                        name: "demuxer-cache-state",
                        change: libmpv::events::PropertyData::Node(_mpv_node),
                        ..
                    }) => {
                        //let ranges = seekable_ranges(mpv_node).unwrap();
                        //println!("Seekable ranges updated: {:?}", ranges);
                    }
                    Ok(libmpv::events::Event::StartFile) => {
                        println!("START");
                    }
                    Ok(e) => {
                        println!("Event triggered: {:?}", e);
                    }
                    Err(e) => println!("Event errored: {:?}", e),
                }
            });
        })
        .unwrap();
    }

    fn prepare_next_to_play(&mut self) {
        self.get_related_video_url();
        self.music_to_play = self.url_into_video_id(&self.music_to_play);
        self.played_video_ids.push(self.music_to_play.clone());
        self.get_audio_url();
    }

    fn get_audio_url(&mut self) {
        let request_url = format!(
            "{}/streams/{}",
            self.piped_api_domains[self.piped_api_domain_index], self.music_to_play
        );
        let mut response: serde_json::Value = reqwest::blocking::get(&request_url)
            .unwrap()
            .json()
            .unwrap();
        let audio_streams: &mut Vec<serde_json::Value> = response
            .get_mut("audioStreams")
            .unwrap()
            .as_array_mut()
            .unwrap();
        audio_streams.sort_by_key(|x| x.get("bitrate").unwrap().as_u64().unwrap());
        let audio_stream = audio_streams.last().unwrap();
        let music_url = audio_stream.get("url").unwrap();
        self.music_to_play = music_url.to_string();
        self.music_to_play = self.music_to_play.replace("\"", "");
        let music_title = response.get("title").unwrap();
        println!("{music_title}");
        println!("----- ");
    }

    fn get_related_video_url(&mut self) {
        let request_url = format!(
            "{}/streams/{}",
            self.piped_api_domains[self.piped_api_domain_index],
            self.played_video_ids.last().unwrap()
        );
        let mut response: serde_json::Value = reqwest::blocking::get(&request_url)
            .unwrap()
            .json()
            .unwrap();
        let related_streams: &mut Vec<serde_json::Value> = response
            .get_mut("relatedStreams")
            .unwrap()
            .as_array_mut()
            .unwrap();

        for related_stream in related_streams {
            let related_video_url = related_stream.get("url").unwrap();
            let related_video_url = related_video_url.as_str().unwrap();
            if self.check_filters_for_related_video_url(related_video_url, related_stream) {
                println!("Next to play: {related_video_url}");
                self.music_to_play = related_video_url.to_string();
                break;
            }
        }
    }

    fn check_filters_for_related_video_url(
        &self,
        video_url: &str,
        stream_json: &serde_json::Value,
    ) -> bool {
        let video_id = self.url_into_video_id(video_url);
        if self.played_video_ids.contains(&video_id) {
            return false;
        } else if stream_json.get("duration").unwrap().as_u64().unwrap() > self.duration_limit {
            return false;
        } else if !self.get_video_genre(&video_id).contains("Music") {
            return false;
        }

        true
    }

    fn get_video_genre(&self, video_id: &str) -> String {
        let request_url = format!(
            "{}/api/v1/videos/{}",
            self.invidious_api_domains[self.invidious_api_domain_index], video_id
        );
        let response: serde_json::Value = reqwest::blocking::get(&request_url)
            .unwrap()
            .json()
            .unwrap();
        let genre: String = response.get("genre").unwrap().as_str().unwrap().to_string();

        genre
    }

    fn url_into_video_id(&self, url: &str) -> String {
        let split = url.split("v=");
        let id = split.last().unwrap().to_string();

        id
    }
}
/*

fn seekable_ranges(demuxer_cache_state: &libmpv::MpvNode) -> Option<Vec<(f64, f64)>> {
    let mut res = Vec::new();
    let props: std::collections::HashMap<&str, libmpv::MpvNode> =
        demuxer_cache_state.to_map()?.collect();
    let ranges = props.get("seekable-ranges")?.to_array()?;

    for node in ranges {
        let range: std::collections::HashMap<&str, libmpv::MpvNode> = node.to_map()?.collect();
        let start = range.get("start")?.to_f64()?;
        let end = range.get("end")?.to_f64()?;
        res.push((start, end));
    }

    Some(res)
}
*/
