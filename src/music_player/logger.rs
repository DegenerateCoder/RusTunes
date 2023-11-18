use crate::music_player::error::Error;
use std::io::Write;

#[derive(Clone)]
pub struct LogSender {
    sender: crossbeam::channel::Sender<LogSignals>,
}

#[derive(Debug)]
pub enum LogSignals {
    Message(String),
    Quit,
}

impl LogSender {
    pub fn new(sender: crossbeam::channel::Sender<LogSignals>) -> Self {
        Self { sender }
    }

    pub fn send_log_message(&self, msg: String) {
        let send = &self.sender;
        send.send(LogSignals::Message(msg)).unwrap();
    }

    pub fn send_quit_signal(&self) {
        let send = &self.sender;
        send.send(LogSignals::Quit).unwrap();
    }
}

pub struct Logger {
    logger_signal_recv: crossbeam::channel::Receiver<LogSignals>,
    logger_signal_send: crossbeam::channel::Sender<LogSignals>,
}

impl Logger {
    pub fn new() -> Self {
        let (s, r) = crossbeam::channel::unbounded();

        Self {
            logger_signal_recv: r,
            logger_signal_send: s,
        }
    }

    pub fn get_signal_send(&self) -> crossbeam::channel::Sender<LogSignals> {
        return self.logger_signal_send.clone();
    }

    pub fn log(&self) -> Result<(), Error> {
        loop {
            let recv = &self.logger_signal_recv;
            if let Ok(signal) = recv.recv() {
                match signal {
                    LogSignals::Message(msg) => self.log_to_file(&msg)?,
                    LogSignals::Quit => break,
                }
            }
        }
        Ok(())
    }

    fn log_to_file(&self, message: &str) -> Result<(), Error> {
        let mut log_file = std::fs::OpenOptions::new()
            .append(true)
            .create(true)
            .open("log.txt")
            .unwrap();

        writeln!(log_file, "{}", message)?;

        Ok(())
    }

    pub fn conditional_log(message: &str, logging_enabled: bool) -> Result<(), Error> {
        if logging_enabled {
            let mut log_file = std::fs::OpenOptions::new()
                .append(true)
                .create(true)
                .open("log.txt")
                .unwrap();

            let timestamp = chrono::Utc::now();
            writeln!(log_file, "{}: {}", timestamp, message)?;
        }
        Ok(())
    }

    pub fn flush(&self) -> Result<(), Error> {
        let recv = &self.logger_signal_recv;
        for _i in 0..recv.len() {
            let signal = recv.recv().unwrap();
            match signal {
                LogSignals::Message(msg) => self.log_to_file(&msg)?,
                _ => (),
            }
        }

        Ok(())
    }
}

impl log::Log for LogSender {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.target().starts_with("rustunes")
    }

    fn log(&self, record: &log::Record) {
        if self.enabled(record.metadata()) {
            let timestamp = chrono::Utc::now();
            let msg = format!("{} {}: {}", timestamp, record.level(), record.args());
            self.send_log_message(msg.to_owned());
        }
    }

    fn flush(&self) {}
}
