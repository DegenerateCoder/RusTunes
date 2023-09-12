use crate::music_player::libmpv_handlers::LibMpvSignals;
use crate::music_player::music_player_core::MusicPlayerLogicSignals;

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
    PlaybackStart,
    PlaybackPause,
    PlaybackResume,
    UpdateTitle(String),
    UpdateDuration(u64),
    UpdateState(TuiState),
    UpdateVolume(i64),
    ModifyScroll(i16),
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

    pub fn draw(&mut self, text: &str, scroll: u16) {
        self.terminal
            .draw(|f| {
                let size = f.size();
                let block = Block::default().title("RusTunes").borders(Borders::ALL);
                let block = block.title_alignment(ratatui::layout::Alignment::Center);
                let text = ratatui::widgets::Paragraph::new(text);
                let text = text.scroll((scroll, 0));
                let inner = block.inner(f.size());
                f.render_widget(block, size);
                f.render_widget(text, inner);
            })
            .unwrap();
    }

    pub fn handle_signals(&mut self) {
        let mut title = "".to_string();
        let mut history: Vec<String> = Vec::new();
        let mut duration = 0;
        let mut playback_start = std::time::SystemTime::now();
        let mut playback_start_offset = 0.0;
        let mut playback_paused = false;
        let mut scroll: u16 = 0;

        loop {
            std::thread::sleep(std::time::Duration::from_millis(100));
            if let Some(recv) = &self.tui_signal_recv {
                if let Ok(signal) = recv.try_recv() {
                    match signal {
                        TuiSignals::PlaybackStart => {
                            playback_start = std::time::SystemTime::now();
                            playback_start_offset = 0.0;
                        }
                        TuiSignals::PlaybackPause => {
                            playback_start_offset +=
                                playback_start.elapsed().unwrap().as_secs_f64();
                            playback_paused = true;
                        }
                        TuiSignals::PlaybackResume => {
                            playback_start = std::time::SystemTime::now();
                            playback_paused = false;
                        }
                        TuiSignals::UpdateTitle(t) => {
                            title = t.clone();
                            let t = t.replace('\n', " ");
                            let mut contains = false;
                            history.iter().for_each(|entry| {
                                contains = contains || entry.contains(&t);
                            });
                            if !contains {
                                history.push(format!("{}: {}", history.len(), t));
                            }
                        }
                        TuiSignals::UpdateState(state) => {
                            self.tui_state = state;
                        }
                        TuiSignals::UpdateVolume(volume) => {
                            self.volume = volume;
                        }
                        TuiSignals::UpdateDuration(dur) => {
                            duration = dur;
                        }
                        TuiSignals::End => {
                            break;
                        }
                        TuiSignals::ModifyScroll(x) => {
                            if x > 0 && scroll < (history.len() - 1) as u16 {
                                scroll += 1;
                            } else if x < 0 && scroll > 0 {
                                scroll -= 1;
                            }
                        }
                    }
                }
            }
            match self.tui_state {
                TuiState::Player => {
                    let playback_time = {
                        if playback_paused {
                            playback_start_offset
                        } else {
                            playback_start_offset + playback_start.elapsed().unwrap().as_secs_f64()
                        }
                    };
                    let mut playback_time = playback_time.ceil() as u64;
                    playback_time = playback_time.min(duration);
                    let symbol = {
                        if playback_paused {
                            "|"
                        } else {
                            ">"
                        }
                    };
                    let mut to_draw = title.clone();
                    to_draw.push_str(&format!(
                        "\n{} {} / {} vol: {}",
                        symbol, playback_time, duration, self.volume
                    ));
                    self.draw(&to_draw, 0);
                }
                TuiState::History => {
                    let mut to_draw = "".to_string();
                    history
                        .iter()
                        .for_each(|x| to_draw.push_str(&format!("{x}\n")));
                    self.draw(&to_draw, scroll);
                }
            }
        }
    }
}

pub struct TUIUserInputHandler {
    tui_state: TuiState,
    volume: i64,
}

impl TUIUserInputHandler {
    pub fn new(volume: i64) -> Self {
        TUIUserInputHandler {
            tui_state: TuiState::Player,
            volume,
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
                self.tui_state = TuiState::Player;
            }
            crossterm::event::KeyCode::Char('2') => {
                tui_signal_send
                    .send(TuiSignals::UpdateState(TuiState::History))
                    .unwrap();
                self.tui_state = TuiState::History;
            }
            crossterm::event::KeyCode::Char(' ') => {
                libmpv_signal_send.send(LibMpvSignals::PauseResume).unwrap();
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
            crossterm::event::KeyCode::Char('b') => {
                libmpv_signal_send.send(LibMpvSignals::PlayNext).unwrap();
            }
            crossterm::event::KeyCode::Char('z') => {
                mp_logic_signal_send
                    .send(MusicPlayerLogicSignals::PlayPrev)
                    .unwrap();
                libmpv_signal_send.send(LibMpvSignals::PlayPrev).unwrap();
            }

            _ => (),
        }

        match self.tui_state {
            TuiState::Player => (),
            TuiState::History => self.handle_history_specific_keys(key, tui_signal_send),
        }

        false
    }

    fn handle_history_specific_keys(
        &mut self,
        key: crossterm::event::KeyEvent,
        tui_signal_send: &crossbeam::channel::Sender<TuiSignals>,
    ) {
        match key.code {
            crossterm::event::KeyCode::Char('j') => {
                tui_signal_send.send(TuiSignals::ModifyScroll(1)).unwrap();
            }
            crossterm::event::KeyCode::Char('k') => {
                tui_signal_send.send(TuiSignals::ModifyScroll(-1)).unwrap();
            }
            _ => (),
        }
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
