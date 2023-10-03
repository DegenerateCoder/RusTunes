use crate::music_player::libmpv_handlers::LibMpvSignals;
use crate::music_player::logger;
use crate::music_player::music_player_core::MusicPlayerLogicSignals;
use crate::music_player::tui::commands::{Action, TuiCommands};
use crate::music_player::tui::TuiSignals;
use crate::music_player::tui::TuiState;
use crossterm::event;

#[derive(Debug)]
pub enum TuiInputHandlerSignals {
    Quit,
}

pub struct TUIUserInputHandler {
    tui_state: TuiState,
    volume: i64,
    commands: TuiCommands,
    tui_input_handler_signal_recv: Option<crossbeam::channel::Receiver<TuiInputHandlerSignals>>,
    libmpv_signal_send: Option<crossbeam::channel::Sender<LibMpvSignals>>,
    tui_signal_send: Option<crossbeam::channel::Sender<TuiSignals>>,
    mp_logic_signal_send: Option<crossbeam::channel::Sender<MusicPlayerLogicSignals>>,
    log_send: logger::LogSender,
}

impl TUIUserInputHandler {
    pub fn new(volume: i64, log_send: logger::LogSender) -> Self {
        Self {
            tui_state: TuiState::Player,
            volume,
            commands: TuiCommands::new(),
            tui_input_handler_signal_recv: None,
            libmpv_signal_send: None,
            tui_signal_send: None,
            mp_logic_signal_send: None,
            log_send,
        }
    }

    pub fn create_signal_channel(&mut self) -> crossbeam::channel::Sender<TuiInputHandlerSignals> {
        let (s, r) = crossbeam::channel::unbounded();

        self.tui_input_handler_signal_recv = Some(r);

        s
    }

    pub fn set_senders(
        &mut self,
        libmpv_signal_send: crossbeam::channel::Sender<LibMpvSignals>,
        tui_signal_send: crossbeam::channel::Sender<TuiSignals>,
        mp_logic_signal_send: crossbeam::channel::Sender<MusicPlayerLogicSignals>,
    ) {
        self.libmpv_signal_send = Some(libmpv_signal_send);
        self.tui_signal_send = Some(tui_signal_send);
        self.mp_logic_signal_send = Some(mp_logic_signal_send);
    }

    pub fn handle_user_input(&mut self) {
        loop {
            if event::poll(std::time::Duration::from_millis(100)).unwrap() {
                let event = event::read();
                if let Ok(event) = event {
                    match event {
                        event::Event::Key(key) => {
                            if self.handle_key_event(key) {
                                break;
                            }
                        }
                        _ => (),
                    }
                }
            } else {
                if let Some(recv) = &self.tui_input_handler_signal_recv {
                    if let Ok(signal) = recv.try_recv() {
                        self.log_send.send_log_message(format!(
                            "TUIUserInputHandler::handle_user_input -> {:?}",
                            signal
                        ));
                        match signal {
                            TuiInputHandlerSignals::Quit => {
                                break;
                            }
                        }
                    }
                }
            }
        }
    }

    fn handle_key_event(&mut self, key: crossterm::event::KeyEvent) -> bool {
        let libmpv_signal_send = self.libmpv_signal_send.as_ref().unwrap();
        let tui_signal_send = self.tui_signal_send.as_ref().unwrap();
        let mp_logic_signal_send = self.mp_logic_signal_send.as_ref().unwrap();

        let action = self
            .commands
            .map_keycode_to_action(key.code, &self.tui_state);

        if let Some(action) = action {
            self.log_send.send_log_message(format!(
                "TUIUserInputHandler::handle_key_event -> {:?}",
                action
            ));
            match action {
                Action::Quit => {
                    libmpv_signal_send.send(LibMpvSignals::End).unwrap();
                    tui_signal_send.send(TuiSignals::Quit).unwrap();
                    return true;
                }
                Action::ViewPlayer => {
                    tui_signal_send
                        .send(TuiSignals::UpdateState(TuiState::Player))
                        .unwrap();
                    self.tui_state = TuiState::Player;
                }
                Action::ViewHistory => {
                    tui_signal_send
                        .send(TuiSignals::UpdateState(TuiState::History))
                        .unwrap();
                    self.tui_state = TuiState::History;
                }
                Action::PlayerPauseResume => {
                    libmpv_signal_send.send(LibMpvSignals::PauseResume).unwrap();
                }
                Action::Vol(vol) => {
                    self.volume = Self::get_updated_volume(self.volume, vol);
                    tui_signal_send
                        .send(TuiSignals::UpdateVolume(self.volume))
                        .unwrap();

                    libmpv_signal_send
                        .send(LibMpvSignals::SetVolume(self.volume))
                        .unwrap();
                }
                Action::PlayerNext => {
                    libmpv_signal_send.send(LibMpvSignals::PlayNext).unwrap();
                }
                Action::PlayerPrev => {
                    mp_logic_signal_send
                        .send(MusicPlayerLogicSignals::PlayPrev)
                        .unwrap();
                    libmpv_signal_send.send(LibMpvSignals::PlayPrev).unwrap();
                }
                Action::Scroll(y) => {
                    tui_signal_send.send(TuiSignals::ModifyScroll(y)).unwrap();
                }
            }
        }

        false
    }

    fn get_updated_volume(current_volume: i64, change: i64) -> i64 {
        let volume = current_volume + change;
        if volume > 100 {
            100
        } else if volume < 0 {
            0
        } else {
            volume
        }
    }
}
