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
    UpdateTitle(String),
    UpdateState(TuiState),
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
        loop {
            if let Some(recv) = &self.tui_signal_recv {
                if let Ok(signal) = recv.try_recv() {
                    match signal {
                        TuiSignals::UpdateTitle(t) => {
                            title = t.clone();
                            history.push(format!("{}: {}", history.len(), t.replace('\n', " ")));
                        }
                        TuiSignals::UpdateState(state) => {
                            self.tui_state = state;
                        }
                        TuiSignals::End => {
                            break;
                        }
                    }
                }
            }
            match self.tui_state {
                TuiState::Player => {
                    let mut to_draw = title.clone();
                    to_draw.push_str(&format!("\nVol: {}", self.volume));
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

pub fn handle_user_input(
    libmpv_signal_send: &crossbeam::channel::Sender<LibMpvSignals>,
    tui_signal_send: &crossbeam::channel::Sender<TuiSignals>,
) {
    loop {
        let event = event::read();
        if let Ok(event) = event {
            match event {
                event::Event::Key(key) => {
                    if handle_key_event(key, libmpv_signal_send, tui_signal_send) {
                        break;
                    }
                }
                _ => (),
            }
        }
    }
}

fn handle_key_event(
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
        _ => (),
    }

    false
}
