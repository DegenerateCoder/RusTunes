use crate::music_player::music_player_core::MusicPlayerLogicSignals;
use crate::music_player::music_player_tui::TuiSignals;

pub enum LibMpvSignals {
    PlayAudio(String),
    PlayNext,
    PlayPrev,
    PauseResume,
    SetVolume(i64),
    End,
}

pub struct LibMpvHandler {
    mpv: libmpv::Mpv,
    libmpv_signal_recv: Option<crossbeam::channel::Receiver<LibMpvSignals>>,
}

impl LibMpvHandler {
    pub fn initialize_libmpv(volume: i64) -> Result<Self, libmpv::Error> {
        let mpv = libmpv::Mpv::new()?;
        mpv.set_property("volume", volume)?;
        mpv.set_property("vo", "null")?;

        let libmpv_signal_recv = None;

        Ok(LibMpvHandler {
            mpv,
            libmpv_signal_recv,
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
        loop {
            if let Some(recv) = &self.libmpv_signal_recv {
                if let Ok(signal) = recv.recv() {
                    match signal {
                        LibMpvSignals::PlayAudio(source) => {
                            self.mpv
                                .playlist_load_files(&[(
                                    &source,
                                    libmpv::FileState::AppendPlay,
                                    None,
                                )])
                                .unwrap();
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
                    }
                }
            }
        }
    }
}

pub fn libmpv_event_handling(
    mut ev_ctx: libmpv::events::EventContext,
    mp_logic_signal_send: &crossbeam::channel::Sender<MusicPlayerLogicSignals>,
    tui_signal_send: &crossbeam::channel::Sender<TuiSignals>,
) {
    loop {
        let ev = ev_ctx.wait_event(600.).unwrap_or(Err(libmpv::Error::Null));

        match ev {
            Ok(libmpv::events::Event::EndFile(_r)) => {
                mp_logic_signal_send
                    .send(MusicPlayerLogicSignals::PlaybackEnded)
                    .unwrap();
            }
            Ok(libmpv::events::Event::PropertyChange {
                name: "pause",
                change: libmpv::events::PropertyData::Flag(pause),
                ..
            }) => {
                if pause {
                    tui_signal_send.send(TuiSignals::PlaybackPause).unwrap();
                } else {
                    tui_signal_send.send(TuiSignals::PlaybackResume).unwrap();
                }
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
                tui_signal_send.send(TuiSignals::PlaybackStart).unwrap();
            }
            Ok(libmpv::events::Event::Shutdown) => {
                mp_logic_signal_send
                    .send(MusicPlayerLogicSignals::End)
                    .unwrap();
                break;
            }
            Ok(_e) => {
                //println!("Event triggered: {:?}", e);
            }
            Err(_e) => (), // println!("Event errored: {:?}", _e),
        }
    }
}
