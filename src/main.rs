use rustunes::music_player::logger;
use rustunes::music_player::{music_player_config::MusicPlayerOptions, MusicPlayer};

fn main() {
    let logger = logger::Logger::new();
    let log_send = logger::LogSender::new(logger.get_signal_send());

    log::set_boxed_logger(Box::new(log_send.clone())).unwrap();
    log::set_max_level(log::LevelFilter::Trace);

    let mut args: Vec<String> = std::env::args().collect();
    let user_input: Option<String> = {
        if args.len() > 1 {
            args.pop()
        } else {
            None
        }
    };

    crossbeam::scope(|scope| {
        scope.spawn(|_| {
            if let Some(user_input) = user_input {
                let args = &args[1..];
                let mut music_player = MusicPlayer::new(args, log_send);
                music_player.play(&user_input);
            } else {
                MusicPlayerOptions::new().print_help();
            }
        });
        scope.spawn(|_| logger.log());
    })
    .unwrap();

    logger.flush().unwrap();
}
