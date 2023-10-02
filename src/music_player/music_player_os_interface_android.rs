use crate::music_player::libmpv_handlers::LibMpvSignals;
use crate::music_player::music_player_core::MusicPlayerLogicSignals;

#[allow(dead_code)]
pub enum OSInterfaceSignals {
    Pause,
    Resume,
    PlayNext,
    PlayPrev,
    UpdateMetadataTitle(String),
    End,
}

pub struct MediaPlayerOSInterface {
    os_interface_recv: Option<crossbeam::channel::Receiver<OSInterfaceSignals>>,
    libmpv_signal_send: Option<crossbeam::channel::Sender<LibMpvSignals>>,
    mp_logic_signal_send: Option<crossbeam::channel::Sender<MusicPlayerLogicSignals>>,
}

impl MediaPlayerOSInterface {
    pub fn new() -> Self {
        MediaPlayerOSInterface {
            os_interface_recv: None,
            libmpv_signal_send: None,
            mp_logic_signal_send: None,
        }
    }

    pub fn create_signal_channel(&mut self) -> crossbeam::channel::Sender<OSInterfaceSignals> {
        let (s, r) = crossbeam::channel::unbounded();

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
        println!("AAAA");
        loop {
            if let Some(recv) = &self.os_interface_recv {
                if let Ok(signal) = recv.recv() {
                    match signal {
                        OSInterfaceSignals::End => {
                            break;
                        }
                        _ => (),
                    }
                }
            }
        }
    }
}
