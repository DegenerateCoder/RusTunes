pub mod music_source;

use crate::music_player::libmpv_handlers::LibMpvSignals;
use crate::music_player::music_player_config::MusicPlayerConfig;
use crate::music_player::music_player_os_interface::OSInterfaceSignals;
use crate::music_player::tui::{user_input_handler::TuiInputHandlerSignals, TuiSignals};
use crate::music_player::Error;
use rand::seq::SliceRandom;
use rand::thread_rng;
use std::collections::VecDeque;

#[derive(Debug)]
pub enum MusicPlayerLogicSignals {
    PrepareNextFile,
    PlaybackEnded,
    PlayPrev,
    End,
    BrokenUrl,
}

struct SignalSendersCollection {
    libmpv: Option<crossbeam::channel::Sender<LibMpvSignals>>,
    tui: Option<crossbeam::channel::Sender<TuiSignals>>,
    os_interface: Option<crossbeam::channel::Sender<OSInterfaceSignals>>,
    tui_input_handler: Option<crossbeam::channel::Sender<TuiInputHandlerSignals>>,
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
    signals_senders: SignalSendersCollection,
    play_only_recommendations: bool,
}

impl MusicPlayerLogic {
    pub fn new(config: MusicPlayerConfig) -> Result<Self, Error> {
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
            )?,
            mp_logic_signal_recv: None,
            signals_senders: SignalSendersCollection {
                libmpv: None,
                tui: None,
                os_interface: None,
                tui_input_handler: None,
            },
            play_only_recommendations: config.play_only_recommendations,
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
        self.signals_senders.libmpv = Some(libmpv_signal_send);
        self.signals_senders.os_interface = Some(os_interface_signal_send);
        self.signals_senders.tui = Some(tui_signal_send);
        self.signals_senders.tui_input_handler = Some(tui_input_handler_send);
    }

    pub fn validate_user_input(&mut self, user_input: &str) -> Result<(), Error> {
        if user_input.contains("list=") {
            let mut is_valid = self.remote_src_proc.is_valid_playlist_url(user_input);
            while is_valid.is_err() {
                self.handle_piped_api_domain_update()?;
                is_valid = self.remote_src_proc.is_valid_playlist_url(user_input);
            }
            if !is_valid? {
                return Err(Error::InvalidPlaylistUrl(format!(
                    "The provided playlist URL is invalid: {user_input}"
                )));
            }
        } else {
            let mut is_valid = self.remote_src_proc.is_valid_video_url(user_input);
            while is_valid.is_err() {
                self.handle_piped_api_domain_update()?;
                is_valid = self.remote_src_proc.is_valid_video_url(user_input);
            }
            if !is_valid? {
                return Err(Error::InvalidVideoUrl(format!(
                    "The provided video URL is invalid: {user_input}"
                )));
            }
        }
        Ok(())
    }

    pub fn process_user_input(&mut self, user_input: &str) -> Result<(), Error> {
        if user_input.contains("list=") {
            self.playlist_to_play = music_source::Remote::url_into_playlist_id(user_input).unwrap();
            self.prepare_playlist()?;

            if self.play_only_recommendations {
                for music_source in &self.to_play {
                    let remote_src = music_source.get_remote_source()?;
                    self.played_video_ids.push(remote_src.video_id.clone());
                    self.related_queue.push_back(remote_src.video_id.clone());
                }
                self.to_play.clear();

                self.prepare_next_to_play()?;
            }
        } else {
            if self.play_only_recommendations {
                let music_source = music_source::Source::new_remote(user_input).unwrap();

                let remote_src = music_source.get_remote_source()?;
                self.played_video_ids.push(remote_src.video_id.clone());
                self.related_queue.push_back(remote_src.video_id.clone());

                self.prepare_next_to_play()?;
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
            self.handle_piped_api_domain_update()?;
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
        self.prepare_next_to_play()?;
        self.update_tui()?;
        loop {
            if let Some(recv) = &self.mp_logic_signal_recv {
                if let Ok(signal) = recv.recv() {
                    log::info!("MusicPlayerLogic::handle_playback_logic -> {:?}", signal);
                    let os_interface_signal_send =
                        self.signals_senders.os_interface.as_ref().unwrap();
                    match signal {
                        MusicPlayerLogicSignals::PrepareNextFile => {
                            self.prepare_next_to_play()?;
                        }
                        MusicPlayerLogicSignals::PlaybackEnded => {
                            let music_source = self.to_play.get_mut(self.to_play_index).unwrap();
                            let remote_src = music_source.get_remote_source_mut()?;

                            if remote_src.audio_stream_url.is_empty() {
                                self.prepare_next_to_play()?;
                            }
                            self.update_tui()?;
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
                        MusicPlayerLogicSignals::BrokenUrl => {
                            self.handle_piped_api_domain_update()?;
                            self.broken_url()?;
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn broken_url(&mut self) -> Result<(), Error> {
        self.to_play_index -= 1;

        self.fix_broken_url(self.to_play_index)?;

        let libmpv_signal_send = self.signals_senders.libmpv.as_ref().unwrap();
        let music_source = self.to_play.get_mut(self.to_play_index).unwrap();
        let remote_src = music_source.get_remote_source_mut()?;

        libmpv_signal_send
            .send(LibMpvSignals::AddAudio(
                remote_src.audio_stream_url.to_string(),
            ))
            .unwrap();
        libmpv_signal_send
            .send(LibMpvSignals::RemoveBrokenItem(self.to_play_index))
            .unwrap();

        self.to_play_index += 1;

        Ok(())
    }

    fn fix_broken_url(&mut self, broken_index: usize) -> Result<(), Error> {
        let broken_music_source = self.to_play.get_mut(broken_index).unwrap();
        let broken_remote_src = broken_music_source.get_remote_source_mut()?;

        log::info!(
            "MusicPlayerLogic::fix_broken_url -> {:?}::{:?}",
            broken_remote_src.video_id,
            broken_remote_src.title
        );

        let mut music_source = music_source::Source::new_remote(&broken_remote_src.url).unwrap();

        Self::prepare_source(
            &mut music_source,
            &mut self.remote_src_proc,
            &self.signals_senders,
        )?;

        let broken_music_source = self.to_play.get_mut(broken_index).unwrap();
        *broken_music_source = music_source;

        Ok(())
    }

    fn update_tui(&mut self) -> Result<(), Error> {
        let tui_signal_send = self.signals_senders.tui.as_ref().unwrap();
        let os_interface_signal_send = self.signals_senders.os_interface.as_ref().unwrap();

        let music_source = self.to_play.get_mut(self.to_play_index).unwrap();
        let remote_src = music_source.get_remote_source_mut()?;

        log::info!(
            "MusicPlayerLogic::update_tui_data -> {:?}::{:?}",
            remote_src.video_id,
            remote_src.title
        );

        let remote_src = music_source.get_remote_source_mut()?;
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

        self.to_play_index += 1;

        Ok(())
    }

    fn prepare_next_to_play(&mut self) -> Result<(), Error> {
        log::info!("MusicPlayerLogic::prepare_next_to_play",);

        let find_related_source = self.to_play_index == self.to_play.len();
        if find_related_source {
            let related_source = self.find_related_source()?;
            self.to_play.push(related_source);
        }

        let next_to_play_src = self.to_play.get_mut(self.to_play_index).unwrap();
        let next_to_play = next_to_play_src.get_remote_source_mut().unwrap();

        log::info!(
            "MusicPlayerLogic::prepare_next_to_play -> {:?}",
            next_to_play.video_id
        );

        let played = self.played_video_ids.contains(&next_to_play.video_id);
        if !played {
            self.played_video_ids.push(next_to_play.video_id.clone());
        }

        if !self.related_queue.contains(&next_to_play.video_id) {
            self.related_queue.push_back(next_to_play.video_id.clone());
        }

        if next_to_play.audio_stream_url.is_empty() {
            Self::prepare_source(
                next_to_play_src,
                &mut self.remote_src_proc,
                &self.signals_senders,
            )?;
            let next_to_play = next_to_play_src.get_remote_source_mut().unwrap();
            let libmpv_signal_send = self.signals_senders.libmpv.as_ref().unwrap();
            libmpv_signal_send
                .send(LibMpvSignals::AddAudio(
                    next_to_play.audio_stream_url.to_owned(),
                ))
                .unwrap();
        }

        Ok(())
    }

    fn find_related_source(&mut self) -> Result<music_source::Source, Error> {
        let related_video_id = self.related_queue.pop_front().unwrap();
        self.related_queue.push_back(related_video_id.clone());

        log::info!(
            "MusicPlayerLogic::find_related_source -> {:?}",
            related_video_id
        );

        let mut related_source = self
            .remote_src_proc
            .get_related_video_source(&related_video_id, &self.played_video_ids);

        while related_source.is_err() {
            match related_source.unwrap_err() {
                Error::AllInvidiousApiDomainsDown(_) => Self::invidious_api_domains_error(
                    &mut self.remote_src_proc,
                    &self.signals_senders,
                )?,
                _ => self.handle_piped_api_domain_update()?,
            }

            related_source = self
                .remote_src_proc
                .get_related_video_source(&related_video_id, &self.played_video_ids);
        }

        Ok(related_source?)
    }

    fn prepare_source(
        music_src: &mut music_source::Source,
        remote_src_proc: &mut music_source::RemoteSourceProcessor,
        signals_senders: &SignalSendersCollection,
    ) -> Result<(), Error> {
        let remote_src = music_src.get_remote_source_mut().unwrap();
        log::info!(
            "MusicPlayerLogic::prepare_source -> {:?}::{:?}",
            remote_src.video_id,
            remote_src.title
        );

        let error = Self::prepare_source_impl(music_src, remote_src_proc);

        if let Err(_err) = error {
            Self::piped_api_domains_error(remote_src_proc, signals_senders)?;
            Self::prepare_source_impl(music_src, remote_src_proc)?;
        }

        Ok(())
    }

    fn prepare_source_impl(
        music_src: &mut music_source::Source,
        remote_src_proc: &mut music_source::RemoteSourceProcessor,
    ) -> Result<(), Error> {
        let music_src = music_src.get_remote_source_mut()?;

        while remote_src_proc.set_audio_url_title(music_src).is_err() {
            remote_src_proc.next_piped_api_domains_index()?;
        }

        Ok(())
    }

    fn invidious_api_domains_error(
        remote_src_proc: &mut music_source::RemoteSourceProcessor,
        signals_senders: &SignalSendersCollection,
    ) -> Result<(), Error> {
        let libmpv_signal_send = signals_senders.libmpv.as_ref().unwrap();
        let os_interface_signal_send = signals_senders.os_interface.as_ref().unwrap();
        let tui_signal_send = signals_senders.tui.as_ref().unwrap();
        let tui_input_handler_send = signals_senders.tui_input_handler.as_ref().unwrap();

        log::info!("MusicPlayerLogic::invidious_api_domains_error");

        let result = remote_src_proc.fetch_invidious_api_domains();
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

    fn piped_api_domains_error(
        remote_src_proc: &mut music_source::RemoteSourceProcessor,
        signals_senders: &SignalSendersCollection,
    ) -> Result<(), Error> {
        let libmpv_signal_send = signals_senders.libmpv.as_ref().unwrap();
        let os_interface_signal_send = signals_senders.os_interface.as_ref().unwrap();
        let tui_signal_send = signals_senders.tui.as_ref().unwrap();
        let tui_input_handler_send = signals_senders.tui_input_handler.as_ref().unwrap();

        log::info!("MusicPlayerLogic::piped_api_domains_error");

        let result = remote_src_proc.fetch_piped_api_domains();
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

    pub fn send_quit_signals(&self) {
        let signals_senders = &self.signals_senders;
        let libmpv_signal_send = signals_senders.libmpv.as_ref().unwrap();
        let os_interface_signal_send = signals_senders.os_interface.as_ref().unwrap();
        let tui_signal_send = signals_senders.tui.as_ref().unwrap();
        let tui_input_handler_send = signals_senders.tui_input_handler.as_ref().unwrap();

        libmpv_signal_send.send(LibMpvSignals::End).unwrap();
        tui_signal_send.send(TuiSignals::Quit).unwrap();
        tui_input_handler_send
            .send(TuiInputHandlerSignals::Quit)
            .unwrap();
        os_interface_signal_send
            .send(OSInterfaceSignals::End)
            .unwrap();
    }

    fn handle_piped_api_domain_update(&mut self) -> Result<(), Error> {
        let next_index_result = self.remote_src_proc.next_piped_api_domains_index();
        if next_index_result.is_err() {
            Self::piped_api_domains_error(&mut self.remote_src_proc, &self.signals_senders)?;
        }

        Ok(())
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
