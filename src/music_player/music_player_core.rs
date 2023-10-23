pub mod music_source;

use crate::music_player::libmpv_handlers::LibMpvSignals;
use crate::music_player::logger::{Error, LogSender};
use crate::music_player::music_player_config::MusicPlayerConfig;
use crate::music_player::music_player_os_interface::OSInterfaceSignals;
use crate::music_player::tui::{user_input_handler::TuiInputHandlerSignals, TuiSignals};
use rand::seq::SliceRandom;
use rand::thread_rng;
use std::collections::VecDeque;

#[derive(Debug)]
pub enum MusicPlayerLogicSignals {
    PlaybackEnded,
    PlayPrev,
    End,
}

pub struct MusicPlayerLogic {
    to_play: Vec<music_source::Source>,
    to_play_index: usize,
    playlist_to_play: String,
    shuffle_playlist: bool,
    played_video_ids: Vec<String>,
    related_queue: VecDeque<String>,
    remote_src_proc: music_source::RemoteSourceProcessor,
    mp_logic_signal_recv: Option<crossbeam::channel::Receiver<MusicPlayerLogicSignals>>,
    libmpv_signal_send: Option<crossbeam::channel::Sender<LibMpvSignals>>,
    tui_signal_send: Option<crossbeam::channel::Sender<TuiSignals>>,
    os_interface_signal_send: Option<crossbeam::channel::Sender<OSInterfaceSignals>>,
    tui_input_handler_send: Option<crossbeam::channel::Sender<TuiInputHandlerSignals>>,
    play_only_recommendations: bool,
    log_send: LogSender,
}

impl MusicPlayerLogic {
    pub fn new(config: MusicPlayerConfig, log_send: LogSender) -> Result<Self, Error> {
        Ok(MusicPlayerLogic {
            to_play: Vec::new(),
            to_play_index: 0,
            playlist_to_play: "".to_string(),
            shuffle_playlist: config.shuffle_playlist,
            played_video_ids: Vec::new(),
            related_queue: VecDeque::new(),
            remote_src_proc: music_source::RemoteSourceProcessor::new(
                config.piped_api_domains,
                config.piped_api_domain_index,
                config.invidious_api_domains,
                config.invidious_api_domain_index,
                config.video_duration_limit_s,
                log_send.clone(),
            )?,
            mp_logic_signal_recv: None,
            libmpv_signal_send: None,
            tui_signal_send: None,
            os_interface_signal_send: None,
            tui_input_handler_send: None,
            play_only_recommendations: config.play_only_recommendations,
            log_send,
        })
    }

    pub fn create_signal_channel(&mut self) -> crossbeam::channel::Sender<MusicPlayerLogicSignals> {
        let (s, r) = crossbeam::channel::unbounded();

        self.mp_logic_signal_recv = Some(r);

        s
    }

    pub fn set_signal_senders(
        &mut self,
        libmpv_signal_send: crossbeam::channel::Sender<LibMpvSignals>,
        os_interface_signal_send: crossbeam::channel::Sender<OSInterfaceSignals>,
        tui_signal_send: crossbeam::channel::Sender<TuiSignals>,
        tui_input_handler_send: crossbeam::channel::Sender<TuiInputHandlerSignals>,
    ) {
        self.libmpv_signal_send = Some(libmpv_signal_send);
        self.os_interface_signal_send = Some(os_interface_signal_send);
        self.tui_signal_send = Some(tui_signal_send);
        self.tui_input_handler_send = Some(tui_input_handler_send);
    }

    pub fn process_user_input(&mut self, user_input: &str) -> Result<(), Error> {
        if user_input.contains("list=") {
            self.playlist_to_play = music_source::Remote::url_into_playlist_id(user_input).unwrap();
            self.prepare_playlist()?;

            if self.play_only_recommendations {
                let last_to_play = self.to_play.pop().unwrap();
                for music_source in &self.to_play {
                    let remote_src = music_source.get_remote_source()?;
                    self.played_video_ids.push(remote_src.video_id.clone());
                    self.related_queue.push_back(remote_src.video_id.clone());
                }
                self.to_play.clear();

                self.to_play.push(last_to_play);
                self.prepare_next_to_play()?;
                self.to_play_index = 0;
                let recom_music_source = self.to_play.pop().unwrap();
                self.to_play.clear();
                self.to_play.push(recom_music_source);
            }
        } else {
            if self.play_only_recommendations {
                let music_source = music_source::Source::new_remote(user_input).unwrap();

                let remote_src = music_source.get_remote_source()?;
                self.played_video_ids.push(remote_src.video_id.clone());
                self.to_play.push(music_source);

                self.prepare_next_to_play()?;

                self.to_play_index = 0;
                let recom_music_source = self.to_play.pop().unwrap();
                self.to_play.clear();
                self.to_play.push(recom_music_source);
            } else {
                let music_source = music_source::Source::new_remote(user_input).unwrap();
                self.to_play.push(music_source);
            }
        }
        Ok(())
    }

    fn prepare_playlist(&mut self) -> Result<(), Error> {
        let mut to_play = self
            .remote_src_proc
            .playlist_to_remote_vec(&self.playlist_to_play);

        while to_play.is_err() {
            let update = self.remote_src_proc.next_piped_api_domains_index();
            if update.is_err() {
                self.piped_api_domains_error()?;
            }
            to_play = self
                .remote_src_proc
                .playlist_to_remote_vec(&self.playlist_to_play);
        }

        self.to_play = to_play.unwrap();

        if self.shuffle_playlist {
            self.to_play.shuffle(&mut thread_rng());
        }

        Ok(())
    }

    pub fn handle_playback_logic(&mut self) -> Result<(), Error> {
        self.prepare_audio()?;
        self.prepare_next_to_play()?;
        loop {
            if let Some(recv) = &self.mp_logic_signal_recv {
                if let Ok(signal) = recv.recv() {
                    self.log_send.send_log_message(format!(
                        "MusicPlayerLogic::handle_playback_logic -> {:?}",
                        signal
                    ));
                    let os_interface_signal_send = self.os_interface_signal_send.as_ref().unwrap();
                    match signal {
                        MusicPlayerLogicSignals::PlaybackEnded => {
                            self.prepare_audio()?;
                            self.prepare_next_to_play()?;
                        }
                        MusicPlayerLogicSignals::End => {
                            os_interface_signal_send
                                .send(OSInterfaceSignals::End)
                                .unwrap();
                            break;
                        }
                        MusicPlayerLogicSignals::PlayPrev => {
                            if self.to_play_index > 1 {
                                self.to_play_index -= 2;
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn prepare_audio(&mut self) -> Result<(), Error> {
        let libmpv_signal_send = self.libmpv_signal_send.as_ref().unwrap();
        let tui_signal_send = self.tui_signal_send.as_ref().unwrap();
        let os_interface_signal_send = self.os_interface_signal_send.as_ref().unwrap();

        let music_source = self.to_play.get_mut(self.to_play_index).unwrap();
        self.log_send.send_log_message(format!(
            "MusicPlayerLogic::prepare_audio -> {:?}",
            music_source
        ));
        match music_source {
            music_source::Source::Remote(remote_src) => {
                let played = self.played_video_ids.contains(&remote_src.video_id);
                if !played {
                    self.played_video_ids.push(remote_src.video_id.clone());
                    if remote_src.audio_stream_url.is_empty() {
                        while self
                            .remote_src_proc
                            .set_audio_url_title(remote_src)
                            .is_err()
                        {
                            let update = self.remote_src_proc.next_piped_api_domains_index();
                            if update.is_err() {
                                self.played_video_ids.pop().unwrap();
                                self.piped_api_domains_error()?;
                                self.prepare_audio()?;
                                return Ok(());
                            }
                        }
                    }
                }
                tui_signal_send
                    .send(TuiSignals::UpdateTitle(format!(
                        "{}\n{}/{}",
                        remote_src.title.to_string(),
                        self.remote_src_proc.get_piped_api_domain(),
                        remote_src.video_id
                    )))
                    .unwrap();
                tui_signal_send
                    .send(TuiSignals::UpdateDuration(remote_src.length))
                    .unwrap();
                os_interface_signal_send
                    .send(OSInterfaceSignals::UpdateMetadataTitle(
                        remote_src.title.to_string(),
                    ))
                    .unwrap();
                if !played {
                    libmpv_signal_send
                        .send(LibMpvSignals::PlayAudio(
                            remote_src.audio_stream_url.to_string(),
                        ))
                        .unwrap();
                }
            }
            _ => panic!(),
        }

        Ok(())
    }

    fn prepare_next_to_play(&mut self) -> Result<(), Error> {
        let music_source = self.to_play.get_mut(self.to_play_index).unwrap();
        self.log_send.send_log_message(format!(
            "MusicPlayerLogic::prepare_next_to_play -> {:?}",
            music_source
        ));
        match music_source {
            music_source::Source::Remote(remote_src) => {
                if !self.related_queue.contains(&remote_src.video_id) {
                    self.related_queue.push_back(remote_src.video_id.clone());
                }
                if self.to_play_index == self.to_play.len() - 1 {
                    let related_video_id = self.related_queue.pop_front().unwrap();
                    self.related_queue.push_back(related_video_id.clone());

                    let mut next_to_play = self
                        .remote_src_proc
                        .get_related_video_url(&related_video_id, &self.played_video_ids);

                    while next_to_play.is_err() {
                        match next_to_play.unwrap_err() {
                            Error::AllInvidiousApiDomainsDown(_) => {
                                self.invidious_api_domains_error()?
                            }
                            _ => {
                                let update = self.remote_src_proc.next_piped_api_domains_index();
                                if update.is_err() {
                                    self.piped_api_domains_error()?;
                                }
                            }
                        }

                        next_to_play = self
                            .remote_src_proc
                            .get_related_video_url(&related_video_id, &self.played_video_ids);
                    }
                    let mut next_to_play = next_to_play.unwrap();

                    match &mut next_to_play {
                        music_source::Source::Remote(next) => {
                            while self.remote_src_proc.set_audio_url_title(next).is_err() {
                                let update = self.remote_src_proc.next_piped_api_domains_index();
                                if update.is_err() {
                                    self.piped_api_domains_error()?;
                                }
                            }
                        }
                        _ => panic!(),
                    }

                    self.to_play.push(next_to_play);
                }
            }
            _ => panic!(),
        }
        self.to_play_index += 1;

        Ok(())
    }

    fn invidious_api_domains_error(&mut self) -> Result<(), Error> {
        let libmpv_signal_send = self.libmpv_signal_send.as_ref().unwrap();
        let os_interface_signal_send = self.os_interface_signal_send.as_ref().unwrap();
        let tui_signal_send = self.tui_signal_send.as_ref().unwrap();
        let tui_input_handler_send = self.tui_input_handler_send.as_ref().unwrap();

        self.log_send
            .send_log_message(format!("MusicPlayerLogic::invidious_api_domains_error"));

        let result = self.remote_src_proc.fetch_invidious_api_domains();
        if result.is_err() {
            libmpv_signal_send.send(LibMpvSignals::End).unwrap();
            tui_signal_send.send(TuiSignals::Quit).unwrap();
            tui_input_handler_send
                .send(TuiInputHandlerSignals::Quit)
                .unwrap();
            os_interface_signal_send
                .send(OSInterfaceSignals::End)
                .unwrap();
        }

        Ok(result?)
    }

    fn piped_api_domains_error(&mut self) -> Result<(), Error> {
        let libmpv_signal_send = self.libmpv_signal_send.as_ref().unwrap();
        let os_interface_signal_send = self.os_interface_signal_send.as_ref().unwrap();
        let tui_signal_send = self.tui_signal_send.as_ref().unwrap();
        let tui_input_handler_send = self.tui_input_handler_send.as_ref().unwrap();

        self.log_send
            .send_log_message(format!("MusicPlayerLogic::piped_api_domains_error"));

        let result = self.remote_src_proc.fetch_piped_api_domains();
        if result.is_err() {
            libmpv_signal_send.send(LibMpvSignals::End).unwrap();
            tui_signal_send.send(TuiSignals::Quit).unwrap();
            tui_input_handler_send
                .send(TuiInputHandlerSignals::Quit)
                .unwrap();
            os_interface_signal_send
                .send(OSInterfaceSignals::End)
                .unwrap();
        }

        Ok(result?)
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
