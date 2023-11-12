use rustunes::music_player::logger;
use rustunes::music_player::music_player_config::options_registry::Action;
use rustunes::music_player::{
    music_player_config::{MusicPlayerConfig, MusicPlayerOptions},
    MusicPlayer,
};

fn main() {
    let logger = logger::Logger::new();
    let log_send = logger::LogSender::new(logger.get_signal_send());

    log::set_boxed_logger(Box::new(log_send.clone())).unwrap();
    log::set_max_level(log::LevelFilter::Trace);

    let args: Vec<String> = std::env::args().collect();

    let (args, user_input) = MusicPlayerOptions::extract_user_input_url(args);
    let args = &args[..];

    let music_player_options = MusicPlayerOptions::new();
    let options = music_player_options.preprocess_args(args).unwrap();

    if user_input.is_none() || options.contains(&Action::PrintHelp) {
        MusicPlayerOptions::new().print_help();
        return;
    }
    let user_input = user_input.unwrap();

    let config = MusicPlayerConfig::new().map_err(|err| {
        println!("{:?}", err);
        return;
    });

    if let Ok(mut config) = config {
        let options = config.apply_simple_actions(options);
        let debug_log = config.debug_log;

        crossbeam::scope(|scope| {
            scope.spawn(|_| -> Result<(), logger::Error> {
                config.apply_complex_actions(options).map_err(|err| {
                    println!("{:?}", err);
                    log::error!("{:?}", err);
                    log_send.send_quit_signal();
                    err
                })?;
                let mut music_player = MusicPlayer::new(config, log_send);
                music_player.play(&user_input);

                Ok(())
            });
            if debug_log {
                scope.spawn(|_| logger.log());
            }
        })
        .unwrap();
        if debug_log {
            logger.flush().unwrap();
        }
    }
}
