mod music_source;

pub struct MusicPlayer {
    music_source: music_source::Source,
    playlist_to_play: String,
    play_video: bool,
    played_video_ids: Vec<String>,
    remote_src_proc: music_source::RemoteSourceProcessor,
}

impl MusicPlayer {
    pub fn new() -> Self {
        MusicPlayer {
            music_source: music_source::Source::None,
            playlist_to_play: "".to_string(),
            play_video: false,
            played_video_ids: Vec::new(),
            remote_src_proc: music_source::RemoteSourceProcessor {
                piped_api_domains: vec!["https://piped-api.garudalinux.org".to_string()],
                piped_api_domain_index: 0,
                invidious_api_domains: vec!["https://invidious.garudalinux.org".to_string()],
                invidious_api_domain_index: 0,
                duration_limit: 600,
            },
        }
    }
    pub fn play(&mut self, user_input: &str) {
        if user_input.contains("list=") {
            self.playlist_to_play = user_input.to_string();
            self.play_playlist()
        } else if self.play_video {
            self.music_source = music_source::Source::new_remote(user_input);
            self.play_audio_and_video()
        } else {
            self.music_source = music_source::Source::new_remote(user_input);
            self.play_audio();
        }
    }
    fn play_audio(&mut self) {
        match &mut self.music_source {
            music_source::Source::Remote(remote_src) => {
                self.played_video_ids.push(remote_src.video_id.clone());
                self.remote_src_proc.set_audio_url_title(remote_src);
                loop {
                    self.play_music_mpv();
                }
            }
            _ => panic!(),
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
        mpv.set_property("volume", 75).unwrap();
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
            scope.spawn(|_| match &mut self.music_source {
                music_source::Source::Remote(remote_src) => {
                    println!("Playing: {} at {}", remote_src.title, remote_src.video_id);
                    mpv.playlist_load_files(&[(
                        &remote_src.audio_stream_url,
                        libmpv::FileState::AppendPlay,
                        None,
                    )])
                    .unwrap();
                    self.prepare_next_to_play();

                    // Trigger `Event::EndFile`.
                    //mpv.playlist_next_force().unwrap();
                }
                _ => panic!(),
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
        let mut next_to_play: music_source::Source;
        match &mut self.music_source {
            music_source::Source::Remote(remote_src) => {
                next_to_play = self
                    .remote_src_proc
                    .get_related_video_url(remote_src, &self.played_video_ids);
                match &mut next_to_play {
                    music_source::Source::Remote(next_to_play_src) => {
                        self.played_video_ids
                            .push(next_to_play_src.video_id.clone());
                        self.remote_src_proc.set_audio_url_title(next_to_play_src);
                    }
                    _ => panic!(),
                }
            }
            _ => panic!(),
        }
        self.music_source = next_to_play;
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
