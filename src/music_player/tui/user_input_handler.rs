use crate::music_player::libmpv_handlers::LibMpvSignals;
use crate::music_player::music_player_core::MusicPlayerLogicSignals;
use crate::music_player::tui::commands::{commands_registry::Action, TuiCommands};
use crate::music_player::tui::TuiSignals;
use crate::music_player::tui::TuiState;
use crossterm::event;

#[derive(Debug)]
pub enum TuiInputHandlerSignals {
    Quit,
}

pub struct TUIUserInputHandler {
    tui_state: TuiState,
    enter_command_mode: bool,
    command_text: String,
    command_suggestions: Option<Vec<String>>,
    command_suggestions_index: Option<usize>,
    volume: i64,
    commands: TuiCommands,
    tui_input_handler_signal_recv: Option<crossbeam::channel::Receiver<TuiInputHandlerSignals>>,
    libmpv_signal_send: Option<crossbeam::channel::Sender<LibMpvSignals>>,
    tui_signal_send: Option<crossbeam::channel::Sender<TuiSignals>>,
    mp_logic_signal_send: Option<crossbeam::channel::Sender<MusicPlayerLogicSignals>>,
    send_help_str: bool,
}

impl TUIUserInputHandler {
    pub fn new(volume: i64) -> Self {
        Self {
            tui_state: TuiState::Player,
            enter_command_mode: false,
            command_text: "".to_string(),
            volume,
            commands: TuiCommands::new(),
            tui_input_handler_signal_recv: None,
            libmpv_signal_send: None,
            tui_signal_send: None,
            mp_logic_signal_send: None,
            send_help_str: true,
            command_suggestions: None,
            command_suggestions_index: None,
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
                            if !self.enter_command_mode {
                                if self.handle_key_event(key) {
                                    break;
                                }
                            } else {
                                if self.handle_key_event_command(key) {
                                    break;
                                }
                            }
                        }
                        _ => (),
                    }
                }
            } else {
                if let Some(recv) = &self.tui_input_handler_signal_recv {
                    if let Ok(signal) = recv.try_recv() {
                        log::info!("TUIUserInputHandler::handle_user_input -> {:?}", signal);
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

    fn handle_key_event_command(&mut self, key: crossterm::event::KeyEvent) -> bool {
        let tui_signal_send = self.tui_signal_send.as_ref().unwrap();
        match key.code {
            crossterm::event::KeyCode::Tab => {
                if self.command_suggestions.is_none() {
                    let suggestions = self
                        .commands
                        .generate_suggestions(&self.command_text, &self.tui_state);
                    if !suggestions.is_empty() {
                        self.command_suggestions = Some(suggestions);
                    }
                }

                if self.command_suggestions.is_some() {
                    let suggestions = self.command_suggestions.as_ref().unwrap();
                    let mut i = self.command_suggestions_index.map_or(0, |i| i + 1);
                    if i == suggestions.len() {
                        i = 0;
                    }
                    self.command_suggestions_index = Some(i);

                    let suggestion = suggestions.get(i).unwrap().to_owned();
                    self.command_text = suggestion.to_owned();

                    tui_signal_send
                        .send(TuiSignals::SetCommandText(suggestion))
                        .unwrap();
                }
            }
            crossterm::event::KeyCode::BackTab => {
                if self.command_suggestions.is_none() {
                    let suggestions = self
                        .commands
                        .generate_suggestions(&self.command_text, &self.tui_state);
                    if !suggestions.is_empty() {
                        self.command_suggestions = Some(suggestions);
                    }
                }

                if self.command_suggestions.is_some() {
                    let suggestions = self.command_suggestions.as_ref().unwrap();
                    let i = self
                        .command_suggestions_index
                        .map_or(suggestions.len() - 1, |i| {
                            if i != 0 {
                                i - 1
                            } else {
                                suggestions.len() - 1
                            }
                        });
                    self.command_suggestions_index = Some(i);

                    let suggestion = suggestions.get(i).unwrap().to_owned();
                    self.command_text = suggestion.to_owned();

                    tui_signal_send
                        .send(TuiSignals::SetCommandText(suggestion))
                        .unwrap();
                }
            }
            crossterm::event::KeyCode::Enter => {
                self.command_suggestions_index = None;
                self.command_suggestions = None;

                self.enter_command_mode = false;
                tui_signal_send
                    .send(TuiSignals::EnterCommandMode(false))
                    .unwrap();

                let action = self
                    .commands
                    .map_command_text_to_action(&self.command_text, &self.tui_state);
                self.command_text = "".to_owned();

                if let Some(action) = action {
                    return self.handle_action(action);
                }
            }
            crossterm::event::KeyCode::Char(c) => {
                self.command_suggestions_index = None;
                self.command_suggestions = None;

                self.command_text.push(c);
                tui_signal_send
                    .send(TuiSignals::UpdateCommandText(c))
                    .unwrap();
            }
            crossterm::event::KeyCode::Backspace => {
                self.command_suggestions_index = None;
                self.command_suggestions = None;

                self.command_text.pop();
                tui_signal_send
                    .send(TuiSignals::UpdateCommandTextBackspace)
                    .unwrap();
            }
            _ => {
                self.command_suggestions_index = None;
                self.command_suggestions = None;
            }
        }
        false
    }

    fn handle_key_event(&mut self, key: crossterm::event::KeyEvent) -> bool {
        let action = self
            .commands
            .map_keycode_to_action(key.code, &self.tui_state);

        if let Some(action) = action {
            return self.handle_action(action);
        }
        false
    }

    fn handle_action(&mut self, action: Action) -> bool {
        let libmpv_signal_send = self.libmpv_signal_send.as_ref().unwrap();
        let tui_signal_send = self.tui_signal_send.as_ref().unwrap();
        let mp_logic_signal_send = self.mp_logic_signal_send.as_ref().unwrap();

        log::info!("TUIUserInputHandler::handle_key_event -> {:?}", action);
        match action {
            Action::Quit => {
                mp_logic_signal_send
                    .send(MusicPlayerLogicSignals::End)
                    .unwrap();
                libmpv_signal_send.send(LibMpvSignals::End).unwrap();
                tui_signal_send.send(TuiSignals::Quit).unwrap();

                return true;
            }
            Action::View(tui_state) => {
                if (tui_state == TuiState::Help) && self.send_help_str {
                    self.send_help_str = false;
                    let help_str = self.commands.generate_help_str();
                    tui_signal_send
                        .send(TuiSignals::UpdateHelpStr(help_str))
                        .unwrap();
                }
                tui_signal_send
                    .send(TuiSignals::UpdateState(tui_state.clone()))
                    .unwrap();
                self.tui_state = tui_state;
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
            Action::EnterCommandMode => {
                self.command_text = "".to_string();
                self.enter_command_mode = true;
                tui_signal_send
                    .send(TuiSignals::EnterCommandMode(true))
                    .unwrap();
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
