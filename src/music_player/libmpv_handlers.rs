use crate::music_player::logger::LogSender;
use crate::music_player::music_player_core::MusicPlayerLogicSignals;
use crate::music_player::tui::TuiSignals;

#[derive(Debug)]
pub enum LibMpvSignals {
    RemoveBrokenItem,
    PlayAudio(String),
    PlayNext,
    PlayPrev,
    PauseResume,
    SetVolume(i64),
    End,
}

const MPV_ERROR_LOADING_FAILED: libmpv::Error = libmpv::Error::Raw(-13);

pub struct LibMpvHandler {
    mpv: libmpv::Mpv,
    libmpv_signal_recv: Option<crossbeam::channel::Receiver<LibMpvSignals>>,
    log_send: LogSender,
}

impl LibMpvHandler {
    pub fn initialize_libmpv(volume: i64, log_send: LogSender) -> Result<Self, libmpv::Error> {
        let mpv = libmpv::Mpv::new()?;
        mpv.set_property("volume", volume)?;
        mpv.set_property("vo", "null")?;

        let libmpv_signal_recv = None;

        Ok(LibMpvHandler {
            mpv,
            libmpv_signal_recv,
            log_send,
        })
    }

    pub fn create_event_context(&self) -> Result<libmpv::events::EventContext, libmpv::Error> {
        let ev_ctx = self.mpv.create_event_context();
        ev_ctx.disable_deprecated_events()?;

        ev_ctx
            .observe_property("pause", libmpv::Format::Flag, 0)
            .unwrap();

        Ok(ev_ctx)
    }

    pub fn create_signal_channel(&mut self) -> crossbeam::channel::Sender<LibMpvSignals> {
        let (s, r) = crossbeam::channel::unbounded();

        self.libmpv_signal_recv = Some(r);

        s
    }

    pub fn handle_signals(&self) {
        let mut err = 0;
        loop {
            if let Some(recv) = &self.libmpv_signal_recv {
                if let Ok(signal) = recv.recv() {
                    self.log_send
                        .send_log_message(format!("LibMpvHandler::handle_signals -> {:?}", signal));
                    match signal {
                        LibMpvSignals::PlayAudio(mut source) => {
                            if err == 2 {
                                source = "A".to_owned();
                            }
                            self.mpv
                                .playlist_load_files(&[(
                                    &source,
                                    libmpv::FileState::AppendPlay,
                                    None,
                                )])
                                .unwrap();
                            err += 1;
                        }
                        LibMpvSignals::PauseResume => {
                            let mut pause: bool = self.mpv.get_property("pause").unwrap();
                            pause = !pause;
                            self.mpv.set_property("pause", pause).unwrap();
                        }
                        LibMpvSignals::PlayNext => {
                            self.mpv.playlist_next_force().unwrap();
                        }
                        LibMpvSignals::PlayPrev => {
                            let _ = self.mpv.playlist_previous_weak();
                        }
                        LibMpvSignals::SetVolume(vol) => {
                            self.mpv.set_property("volume", vol).unwrap();
                        }
                        LibMpvSignals::End => {
                            self.mpv.command("quit", &["0"]).unwrap();
                            break;
                        }
                        LibMpvSignals::RemoveBrokenItem => {
                            let count = self.mpv.get_property::<i64>("playlist-count").unwrap();
                            let count: usize = count.try_into().unwrap();
                            self.mpv.playlist_remove_index(count - 2).unwrap();
                        }
                    }
                }
            }
        }
    }
}

pub struct EventHandler {
    mp_logic_signal_send: crossbeam::channel::Sender<MusicPlayerLogicSignals>,
    tui_signal_send: crossbeam::channel::Sender<TuiSignals>,
    log_send: LogSender,
}

impl EventHandler {
    pub fn new(
        mp_logic_signal_send: crossbeam::channel::Sender<MusicPlayerLogicSignals>,
        tui_signal_send: crossbeam::channel::Sender<TuiSignals>,
        log_send: LogSender,
    ) -> Self {
        Self {
            mp_logic_signal_send,
            tui_signal_send,
            log_send,
        }
    }

    pub fn libmpv_event_handling(&self, mut ev_ctx: libmpv::events::EventContext) {
        loop {
            let ev = ev_ctx.wait_event(600.).unwrap_or(Err(libmpv::Error::Null));

            match ev {
                Ok(event) => {
                    self.log_send.send_log_message(format!(
                        "EventHandler::libmpv_event_handling -> {:?}",
                        event
                    ));
                    let end = self.handle_event(event);
                    if end {
                        break;
                    }
                }
                Err(err) => {
                    self.log_send.send_log_message(format!(
                        "EventHandler::libmpv_event_handling -> Error::{:?}",
                        err
                    ));

                    match err {
                        MPV_ERROR_LOADING_FAILED => {
                            self.mp_logic_signal_send
                                .send(MusicPlayerLogicSignals::BrokenUrl)
                                .unwrap();
                        }
                        _ => (),
                    }
                }
            }
        }
    }

    fn handle_event(&self, event: libmpv::events::Event) -> bool {
        match event {
            libmpv::events::Event::EndFile(_r) => {
                self.tui_signal_send.send(TuiSignals::End).unwrap();
                self.mp_logic_signal_send
                    .send(MusicPlayerLogicSignals::PlaybackEnded)
                    .unwrap();
            }
            libmpv::events::Event::PropertyChange {
                name: "pause",
                change: libmpv::events::PropertyData::Flag(pause),
                ..
            } => {
                if pause {
                    self.tui_signal_send
                        .send(TuiSignals::PlaybackPause)
                        .unwrap();
                } else {
                    self.tui_signal_send
                        .send(TuiSignals::PlaybackResume)
                        .unwrap();
                }
            }

            libmpv::events::Event::PropertyChange {
                name: "demuxer-cache-state",
                change: libmpv::events::PropertyData::Node(_mpv_node),
                ..
            } => {
                //let ranges = seekable_ranges(mpv_node).unwrap();
                //println!("Seekable ranges updated: {:?}", ranges);
            }
            libmpv::events::Event::StartFile => {
                self.tui_signal_send.send(TuiSignals::Start).unwrap();
            }
            libmpv::events::Event::PlaybackRestart => {
                self.tui_signal_send.send(TuiSignals::AudioReady).unwrap();
            }
            libmpv::events::Event::Shutdown => {
                self.log_send.send_quit_signal();
                return true;
            }
            _e => (),
        }

        false
    }
}
