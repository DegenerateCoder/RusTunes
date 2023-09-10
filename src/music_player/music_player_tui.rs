use crate::music_player::libmpv_handlers::LibMpvSignals;

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    widgets::{Block, Borders},
    Terminal,
};

pub enum TuiSignals {
    PlaybackPause,
    PlaybackResume,
    UpdateTitle(String),
    UpdateState(TuiState),
    UpdateVolume(i64),
    End,
}

pub enum TuiState {
    Player,
    History,
}

pub struct MusicPlayerTUI {
    terminal: Terminal<CrosstermBackend<std::io::Stdout>>,
    tui_signal_recv: Option<crossbeam::channel::Receiver<TuiSignals>>,
    tui_state: TuiState,
    volume: i64,
}

impl MusicPlayerTUI {
    pub fn setup_terminal(volume: i64) -> Self {
        enable_raw_mode().unwrap();
        let mut stdout = std::io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture).unwrap();
        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend).unwrap();

        MusicPlayerTUI {
            terminal,
            tui_signal_recv: None,
            tui_state: TuiState::Player,
            volume,
        }
    }

    pub fn create_signal_channel(&mut self) -> crossbeam::channel::Sender<TuiSignals> {
        let (s, r) = crossbeam::channel::unbounded();

        self.tui_signal_recv = Some(r);

        s
    }

    pub fn restore_terminal(&mut self) {
        disable_raw_mode().unwrap();
        execute!(
            self.terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )
        .unwrap();
        self.terminal.show_cursor().unwrap();
    }

    pub fn draw(&mut self, text: &str) {
        self.terminal
            .draw(|f| {
                let size = f.size();
                let block = Block::default().title("RusTunes").borders(Borders::ALL);
                let block = block.title_alignment(ratatui::layout::Alignment::Center);
                let text = ratatui::widgets::Paragraph::new(text);
                let inner = block.inner(f.size());
                f.render_widget(block, size);
                f.render_widget(text, inner);
            })
            .unwrap();
    }

    pub fn handle_signals(&mut self) {
        let mut title = "".to_string();
        let mut history = Vec::new();
        let mut playback_paused = false;

        loop {
            std::thread::sleep(std::time::Duration::from_millis(100));
            if let Some(recv) = &self.tui_signal_recv {
                if let Ok(signal) = recv.try_recv() {
                    match signal {
                        TuiSignals::PlaybackPause => {
                            playback_paused = true;
                        }
                        TuiSignals::PlaybackResume => {
                            playback_paused = false;
                        }
                        TuiSignals::UpdateTitle(t) => {
                            title = t.clone();
                            history.push(format!("{}: {}", history.len(), t.replace('\n', " ")));
                        }
                        TuiSignals::UpdateState(state) => {
                            self.tui_state = state;
                        }
                        TuiSignals::UpdateVolume(volume) => {
                            self.volume = volume;
                        }
                        TuiSignals::End => {
                            break;
                        }
                    }
                }
            }
            match self.tui_state {
                TuiState::Player => {
                    let symbol = {
                        if playback_paused {
                            "|"
                        } else {
                            ">"
                        }
                    };
                    let mut to_draw = title.clone();
                    to_draw.push_str(&format!("\n{} vol: {}", symbol, self.volume));
                    self.draw(&to_draw);
                }
                TuiState::History => {
                    let mut to_draw = "".to_string();
                    history.iter().for_each(|x| to_draw.push_str(x));
                    self.draw(&to_draw);
                }
            }
        }
    }
}

pub struct TUIUserInputHandler {
    volume: i64,
    pause: bool,
}

impl TUIUserInputHandler {
    pub fn new(volume: i64) -> Self {
        TUIUserInputHandler {
            volume,
            pause: false,
        }
    }

    pub fn handle_user_input(
        &mut self,
        libmpv_signal_send: &crossbeam::channel::Sender<LibMpvSignals>,
        tui_signal_send: &crossbeam::channel::Sender<TuiSignals>,
    ) {
        loop {
            let event = event::read();
            if let Ok(event) = event {
                match event {
                    event::Event::Key(key) => {
                        if self.handle_key_event(key, libmpv_signal_send, tui_signal_send) {
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
    ) -> bool {
        match key.code {
            crossterm::event::KeyCode::Char('q') => {
                libmpv_signal_send.send(LibMpvSignals::End).unwrap();
                tui_signal_send.send(TuiSignals::End).unwrap();
                return true;
            }
            crossterm::event::KeyCode::Char('1') => {
                tui_signal_send
                    .send(TuiSignals::UpdateState(TuiState::Player))
                    .unwrap();
            }
            crossterm::event::KeyCode::Char('2') => {
                tui_signal_send
                    .send(TuiSignals::UpdateState(TuiState::History))
                    .unwrap();
            }
            crossterm::event::KeyCode::Char(' ') => {
                self.pause = !self.pause;
                if self.pause {
                    libmpv_signal_send.send(LibMpvSignals::Pause).unwrap();
                    tui_signal_send.send(TuiSignals::PlaybackPause).unwrap();
                } else {
                    libmpv_signal_send.send(LibMpvSignals::Resume).unwrap();
                    tui_signal_send.send(TuiSignals::PlaybackResume).unwrap();
                }
            }
            crossterm::event::KeyCode::Char(']') => {
                self.update_volume(10);

                tui_signal_send
                    .send(TuiSignals::UpdateVolume(self.volume))
                    .unwrap();

                libmpv_signal_send
                    .send(LibMpvSignals::SetVolume(self.volume))
                    .unwrap();
            }
            crossterm::event::KeyCode::Char('[') => {
                self.update_volume(-10);

                tui_signal_send
                    .send(TuiSignals::UpdateVolume(self.volume))
                    .unwrap();

                libmpv_signal_send
                    .send(LibMpvSignals::SetVolume(self.volume))
                    .unwrap();
            }
            crossterm::event::KeyCode::Char('}') => {
                self.update_volume(1);

                tui_signal_send
                    .send(TuiSignals::UpdateVolume(self.volume))
                    .unwrap();

                libmpv_signal_send
                    .send(LibMpvSignals::SetVolume(self.volume))
                    .unwrap();
            }
            crossterm::event::KeyCode::Char('{') => {
                self.update_volume(-1);

                tui_signal_send
                    .send(TuiSignals::UpdateVolume(self.volume))
                    .unwrap();

                libmpv_signal_send
                    .send(LibMpvSignals::SetVolume(self.volume))
                    .unwrap();
            }

            _ => (),
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
