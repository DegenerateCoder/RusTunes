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
}

impl MusicPlayer {
    fn new() -> MusicPlayer {
        MusicPlayer {
            music_to_play: "".to_string(),
            playlist_to_play: "".to_string(),
            play_video: false,
            piped_api_domains: vec!["https://piped-api.garudalinux.org".to_string()],
            piped_api_domain_index: 0,
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
        self.url_into_video_id();
        self.get_audio_url();
        self.play_music_mpv();
    }

    fn play_playlist(&self) {
        unimplemented!("playlist playback");
    }

    fn play_audio_and_video(&self) {
        unimplemented!("audio and video playback");
    }

    fn url_into_video_id(&mut self) {
        let split = self.music_to_play.split("v=");
        self.music_to_play = split.last().unwrap().to_string();
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
            .take()
            .unwrap()
            .as_array_mut()
            .unwrap();
        audio_streams.sort_by_key(|x| x.get("bitrate").unwrap().as_u64().unwrap());
        let audio_stream = audio_streams.last().unwrap();
        let music_url = audio_stream.get("url").take().unwrap();
        self.music_to_play = music_url.to_string();
        println!("{}", music_url);
        let music_title = response.get("title").take().unwrap();
        println!("{music_title}");
        println!("----- ");
    }

    fn play_music_mpv(&mut self) {
        let mpv = libmpv::Mpv::new().unwrap();
        mpv.set_property("volume", 15).unwrap();
        mpv.set_property("vo", "null").unwrap();

        let mut ev_ctx = mpv.create_event_context();
        ev_ctx.disable_deprecated_events().unwrap();
        ev_ctx
            .observe_property("volume", libmpv::Format::Int64, 0)
            .unwrap();
        ev_ctx
            .observe_property("demuxer-cache-state", libmpv::Format::Node, 0)
            .unwrap();

        crossbeam::scope(|scope| {
            scope.spawn(|_| {
                self.music_to_play = self.music_to_play.replace("\"", "");
                println!("{}", self.music_to_play);
                mpv.playlist_load_files(&[(
                    &self.music_to_play,
                    libmpv::FileState::AppendPlay,
                    None,
                )])
                .unwrap();

                std::thread::sleep(std::time::Duration::from_secs(3));

                mpv.set_property("volume", 100).unwrap();

                std::thread::sleep(std::time::Duration::from_secs(5));

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
                        change: libmpv::events::PropertyData::Node(mpv_node),
                        ..
                    }) => {
                        let ranges = seekable_ranges(mpv_node).unwrap();
                        println!("Seekable ranges updated: {:?}", ranges);
                    }
                    Ok(e) => println!("Event triggered: {:?}", e),
                    Err(e) => println!("Event errored: {:?}", e),
                }
            });
        })
        .unwrap();
    }
}

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

