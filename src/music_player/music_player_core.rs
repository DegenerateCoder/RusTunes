mod music_source;

use crate::music_player::libmpv_handlers::LibMpvSignals;
use crate::music_player::music_player_tui::TuiSignals;
use rand::seq::SliceRandom;
use rand::thread_rng;
use std::collections::VecDeque;

pub enum MusicPlayerLogicSignals {
    PlaybackEnded,
    End,
}

pub struct MusicPlayerLogic {
    next_to_play: VecDeque<music_source::Source>,
    playlist_to_play: String,
    shuffle_playlist: bool,
    played_video_ids: Vec<String>,
    related_queue: VecDeque<String>,
    remote_src_proc: music_source::RemoteSourceProcessor,
    mp_logic_signal_recv: Option<crossbeam::channel::Receiver<MusicPlayerLogicSignals>>,
}

#[derive(serde::Deserialize)]
pub struct MusicPlayerConfig {
    piped_api_domains: Vec<String>,
    piped_api_domain_index: usize,
    shuffle_playlist: bool,
    invidious_api_domains: Vec<String>,
    invidious_api_domain_index: usize,
    pub mpv_base_volume: i64,
    video_duration_limit_s: u64,
}

impl MusicPlayerLogic {
    pub fn new(config: MusicPlayerConfig) -> Self {
        MusicPlayerLogic {
            next_to_play: VecDeque::new(),
            playlist_to_play: "".to_string(),
            shuffle_playlist: config.shuffle_playlist,
            played_video_ids: Vec::new(),
            related_queue: VecDeque::new(),
            remote_src_proc: music_source::RemoteSourceProcessor {
                piped_api_domains: config.piped_api_domains,
                piped_api_domain_index: config.piped_api_domain_index,
                invidious_api_domains: config.invidious_api_domains,
                invidious_api_domain_index: config.invidious_api_domain_index,
                duration_limit: config.video_duration_limit_s,
            },
            mp_logic_signal_recv: None,
        }
    }

    pub fn create_signal_channel(&mut self) -> crossbeam::channel::Sender<MusicPlayerLogicSignals> {
        let (s, r) = crossbeam::channel::unbounded();

        self.mp_logic_signal_recv = Some(r);

        s
    }

    pub fn process_user_input(&mut self, user_input: &str) {
        if user_input.contains("list=") {
            self.playlist_to_play = music_source::Remote::url_into_playlist_id(user_input);
            self.prepare_playlist();
        } else {
            let music_source = music_source::Source::new_remote(user_input);
            self.next_to_play.push_back(music_source);
        }
    }

    fn prepare_playlist(&mut self) {
        self.next_to_play = self
            .remote_src_proc
            .playlist_to_remote_vec(&self.playlist_to_play);

        if self.shuffle_playlist {
            self.next_to_play
                .make_contiguous()
                .shuffle(&mut thread_rng());
        }
    }

    pub fn handle_playback_logic(
        &mut self,
        libmpv_signal_send: &crossbeam::channel::Sender<LibMpvSignals>,
        tui_signal_send: &crossbeam::channel::Sender<TuiSignals>,
    ) {
        self.prepare_audio(libmpv_signal_send, tui_signal_send);
        self.prepare_next_to_play();
        loop {
            if let Some(recv) = &self.mp_logic_signal_recv {
                if let Ok(signal) = recv.try_recv() {
                    match signal {
                        MusicPlayerLogicSignals::PlaybackEnded => {
                            self.prepare_audio(libmpv_signal_send, tui_signal_send);
                            self.prepare_next_to_play();
                        }
                        MusicPlayerLogicSignals::End => {
                            break;
                        }
                    }
                }
            }
        }
    }

    fn prepare_audio(
        &mut self,
        libmpv_signal_send: &crossbeam::channel::Sender<LibMpvSignals>,
        tui_signal_send: &crossbeam::channel::Sender<TuiSignals>,
    ) {
        let music_source = self.next_to_play.get_mut(0).unwrap();
        match music_source {
            music_source::Source::Remote(remote_src) => {
                self.played_video_ids.push(remote_src.video_id.clone());
                self.remote_src_proc.set_audio_url_title(remote_src);
                tui_signal_send
                    .send(TuiSignals::UpdateTitle(format!(
                        "{}\n{}/{}",
                        remote_src.title.to_string(),
                        self.remote_src_proc.piped_api_domains
                            [self.remote_src_proc.piped_api_domain_index],
                        remote_src.video_id
                    )))
                    .unwrap();
                libmpv_signal_send
                    .send(LibMpvSignals::PlayAudio(
                        remote_src.audio_stream_url.to_string(),
                    ))
                    .unwrap();
            }
            _ => panic!(),
        }
    }

    fn prepare_next_to_play(&mut self) {
        let music_source = self.next_to_play.pop_front().unwrap();
        let mut next_to_play: music_source::Source;
        match music_source {
            music_source::Source::Remote(remote_src) => {
                self.related_queue.push_back(remote_src.video_id.clone());
                if self.next_to_play.is_empty() {
                    let related_video_id = self.related_queue.pop_front().unwrap();
                    self.related_queue.push_back(related_video_id.clone());

                    next_to_play = self
                        .remote_src_proc
                        .get_related_video_url(&related_video_id, &self.played_video_ids);
                } else {
                    // TODO: rewrite this, highly inefficient
                    next_to_play = self.next_to_play.pop_front().unwrap();
                }
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
        self.next_to_play.push_front(next_to_play);
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
