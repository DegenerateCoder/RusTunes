pub mod commands;
pub mod user_input_handler;

use crate::music_player::logger;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    widgets::{Block, Borders},
    Terminal,
};

#[derive(Debug)]
pub enum TuiSignals {
    Start,
    AudioReady,
    End,
    PlaybackPause,
    PlaybackResume,
    UpdateTitle(String),
    UpdateDuration(u64),
    UpdateState(TuiState),
    UpdateVolume(i64),
    ModifyScroll(i16),
    Quit,
}

#[derive(PartialEq, Eq, Hash, Debug, Clone)]
pub enum TuiState {
    Player,
    History,
}

pub struct MusicPlayerTUI {
    terminal: Terminal<CrosstermBackend<std::io::Stdout>>,
    tui_signal_recv: Option<crossbeam::channel::Receiver<TuiSignals>>,
    tui_state: TuiState,
    volume: i64,
    log_send: logger::LogSender,
}

impl MusicPlayerTUI {
    pub fn setup_terminal(volume: i64, log_send: logger::LogSender) -> Self {
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
            log_send,
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
        let mut playback_paused = true;
        let mut audio_ready = false;
        let mut scroll: u16 = 0;

        loop {
            std::thread::sleep(std::time::Duration::from_millis(100));
            if let Some(recv) = &self.tui_signal_recv {
                if let Ok(signal) = recv.try_recv() {
                    self.log_send.send_log_message(format!(
                        "MusicPlayerTUI::handle_signals -> {:?}",
                        signal
                    ));
                    match signal {
                        TuiSignals::Start => audio_ready = false,
                        TuiSignals::AudioReady => {
                            playback_start = std::time::SystemTime::now();
                            audio_ready = true;
                            playback_paused = false;
                        }
                        TuiSignals::End => {
                            playback_start_offset = 0.0;
                            playback_paused = true;
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
                        TuiSignals::Quit => {
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
                        if !audio_ready {
                            0.0
                        } else if playback_paused {
                            playback_start_offset
                        } else {
                            playback_start_offset + playback_start.elapsed().unwrap().as_secs_f64()
                        }
                    };
                    let mut playback_time = playback_time.ceil() as u64;
                    playback_time = playback_time.min(duration);
                    let symbol = {
                        if !audio_ready {
                            "|"
                        } else if playback_paused {
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
