use crate::music_player::libmpv_handlers::LibMpvSignals;
use crate::music_player::music_player_core::MusicPlayerLogicSignals;
use crate::music_player::tui::commands::{Action, TuiCommands};
use crate::music_player::tui::TuiSignals;
use crate::music_player::tui::TuiState;
use crossterm::event;

pub struct TUIUserInputHandler {
    tui_state: TuiState,
    volume: i64,
    commands: TuiCommands,
}

impl TUIUserInputHandler {
    pub fn new(volume: i64) -> Self {
        Self {
            tui_state: TuiState::Player,
            volume,
            commands: TuiCommands::new(),
        }
    }

    pub fn handle_user_input(
        &mut self,
        libmpv_signal_send: &crossbeam::channel::Sender<LibMpvSignals>,
        tui_signal_send: &crossbeam::channel::Sender<TuiSignals>,
        mp_logic_signal_send: &crossbeam::channel::Sender<MusicPlayerLogicSignals>,
    ) {
        loop {
            let event = event::read();
            if let Ok(event) = event {
                match event {
                    event::Event::Key(key) => {
                        if self.handle_key_event(
                            key,
                            libmpv_signal_send,
                            tui_signal_send,
                            mp_logic_signal_send,
                        ) {
                            break;
                        }
                    }
                    _ => (),
                }
            }
        }
    }

    fn handle_key_event(
        &mut self,
        key: crossterm::event::KeyEvent,
        libmpv_signal_send: &crossbeam::channel::Sender<LibMpvSignals>,
        tui_signal_send: &crossbeam::channel::Sender<TuiSignals>,
        mp_logic_signal_send: &crossbeam::channel::Sender<MusicPlayerLogicSignals>,
    ) -> bool {
        let action = self
            .commands
            .map_keycode_to_action(key.code, &self.tui_state);

        if let Some(action) = action {
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
                    self.update_volume(vol);

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

    fn update_volume(&mut self, change: i64) {
        self.volume += change;
        if self.volume > 100 {
            self.volume = 100;
        } else if self.volume < 0 {
            self.volume = 0;
        }
    }
}
