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
}

impl MediaPlayerOSInterface {
    pub fn new() -> Self {
        MediaPlayerOSInterface {
            os_interface_recv: None,
        }
    }

    pub fn create_signal_channel(&mut self) -> crossbeam::channel::Sender<OSInterfaceSignals> {
        let (s, r) = crossbeam::channel::unbounded();

        self.os_interface_recv = Some(r);

        s
    }

    pub fn handle_signals(
        &mut self,
        _libmpv_signal_send: &crossbeam::channel::Sender<LibMpvSignals>,
        _mp_logic_signal_send: &crossbeam::channel::Sender<MusicPlayerLogicSignals>,
    ) {
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
