use crate::music_player::libmpv_handlers::LibMpvSignals;
use crate::music_player::music_player_core::MusicPlayerLogicSignals;

pub enum OSInterfaceSignals {
    Pause,
    Resume,
    PlayNext,
    PlayPrev,
    UpdateMetadataTitle(String),
    End,
}

pub struct MediaPlayerOSInterface {
    media_controller: souvlaki::MediaControls,
    os_interface_recv: Option<crossbeam::channel::Receiver<OSInterfaceSignals>>,
    libmpv_signal_send: Option<crossbeam::channel::Sender<LibMpvSignals>>,
    mp_logic_signal_send: Option<crossbeam::channel::Sender<MusicPlayerLogicSignals>>,
    #[cfg(target_os = "windows")]
    #[allow(dead_code)]
    dummy_window: windows_async::DummyWindow,
}

impl MediaPlayerOSInterface {
    pub fn new() -> Self {
        #[cfg(not(target_os = "windows"))]
        let hwnd = None;

        #[cfg(target_os = "windows")]
        let dummy_window = windows_async::create_dummy_window();
        #[cfg(target_os = "windows")]
        let hwnd = {
            use std::os::raw::c_void;
            let hwnd = dummy_window.hwnd().0.to_owned() as *mut c_void;
            Some(hwnd)
        };

        let config = souvlaki::PlatformConfig {
            dbus_name: "rustunes",
            display_name: "RusTunes",
            hwnd,
        };

        let media_controller = souvlaki::MediaControls::new(config).unwrap();

        MediaPlayerOSInterface {
            media_controller,
            os_interface_recv: None,
            libmpv_signal_send: None,
            mp_logic_signal_send: None,
            #[cfg(target_os = "windows")]
            dummy_window,
        }
    }

    pub fn create_signal_channel(&mut self) -> crossbeam::channel::Sender<OSInterfaceSignals> {
        let (s, r) = crossbeam::channel::unbounded();
        let signal_sender = s.clone();

        // The closure must be Send and have a static lifetime.
        self.media_controller
            .attach(move |event: souvlaki::MediaControlEvent| match event {
                souvlaki::MediaControlEvent::Play => {
                    signal_sender.send(OSInterfaceSignals::Resume).unwrap();
                }
                souvlaki::MediaControlEvent::Pause => {
                    signal_sender.send(OSInterfaceSignals::Pause).unwrap();
                }
                souvlaki::MediaControlEvent::Next => {
                    signal_sender.send(OSInterfaceSignals::PlayNext).unwrap();
                }
                souvlaki::MediaControlEvent::Previous => {
                    signal_sender.send(OSInterfaceSignals::PlayPrev).unwrap();
                }
                _ => (),
            })
            .unwrap();

        self.media_controller
            .set_playback(souvlaki::MediaPlayback::Playing { progress: None })
            .unwrap();

        self.os_interface_recv = Some(r);

        s
    }

    pub fn set_senders(
        &mut self,
        libmpv_signal_send: crossbeam::channel::Sender<LibMpvSignals>,
        mp_logic_signal_send: crossbeam::channel::Sender<MusicPlayerLogicSignals>,
    ) {
        self.libmpv_signal_send = Some(libmpv_signal_send);
        self.mp_logic_signal_send = Some(mp_logic_signal_send);
    }

    pub fn handle_signals(&mut self) {
        let libmpv_signal_send = self.libmpv_signal_send.as_ref().unwrap();
        let mp_logic_signal_send = self.mp_logic_signal_send.as_ref().unwrap();

        loop {
            if let Some(recv) = &self.os_interface_recv {
                if let Ok(signal) = recv.recv() {
                    match signal {
                        OSInterfaceSignals::Resume => {
                            libmpv_signal_send.send(LibMpvSignals::PauseResume).unwrap();
                            self.media_controller
                                .set_playback(souvlaki::MediaPlayback::Playing { progress: None })
                                .unwrap();
                        }
                        OSInterfaceSignals::Pause => {
                            libmpv_signal_send.send(LibMpvSignals::PauseResume).unwrap();
                            self.media_controller
                                .set_playback(souvlaki::MediaPlayback::Paused { progress: None })
                                .unwrap();
                        }
                        OSInterfaceSignals::PlayNext => {
                            libmpv_signal_send.send(LibMpvSignals::PlayNext).unwrap();
                        }
                        OSInterfaceSignals::PlayPrev => {
                            mp_logic_signal_send
                                .send(MusicPlayerLogicSignals::PlayPrev)
                                .unwrap();
                            libmpv_signal_send.send(LibMpvSignals::PlayPrev).unwrap();
                        }
                        OSInterfaceSignals::UpdateMetadataTitle(title) => {
                            self.media_controller
                                .set_metadata(souvlaki::MediaMetadata {
                                    title: Some(&title),
                                    ..Default::default()
                                })
                                .unwrap();
                        }
                        OSInterfaceSignals::End => {
                            break;
                        }
                    }
                }
            }
        }
    }
}
