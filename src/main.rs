use rustunes::music_player::error::Error;
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
    let args = &args[1..];

    let music_player_options = MusicPlayerOptions::new();
    let options = music_player_options
        .preprocess_args(args)
        .map_err(|err| match err {
            Error::InvalidOption(msg) => println!("{msg}\n"),
            _ => println!("{:?}", err),
        })
        .or::<Result<Vec<Action>, Error>>(Ok(vec![Action::PrintHelp]))
        .unwrap();

    let overwrite_config = options.contains(&Action::OverwriteConfig);

    if options.contains(&Action::PrintHelp) || (user_input.is_none() && !overwrite_config) {
        MusicPlayerOptions::new().print_help();
        return;
    }

    let config = MusicPlayerConfig::new().map_err(|err| {
        println!("{:?}", err);
        return;
    });

    if let Ok(mut config) = config {
        let options = config.apply_simple_actions(options);
        let debug_log = config.debug_log;

        crossbeam::scope(|scope| {
            scope.spawn(|_| -> Result<(), Error> {
                let result = music_player(config, options, overwrite_config, user_input);
                log_send.send_quit_signal();

                result
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

fn music_player(
    mut config: MusicPlayerConfig,
    options: Vec<Action>,
    overwrite_config: bool,
    user_input: Option<String>,
) -> Result<(), Error> {
    config.apply_complex_actions(options).map_err(|err| {
        println!("{:?}", err);
        log::error!("{:?}", err);
        err
    })?;

    config.overwrite_config(overwrite_config).map_err(|err| {
        println!("{:?}", err);
        log::error!("{:?}", err);
        err
    })?;

    if let Some(user_input) = user_input {
        let mut music_player = MusicPlayer::new(config);

        music_player.play(&user_input).map_err(|err| {
            println!("{:?}", err);
            log::error!("{:?}", err);
            err
        })?;
    }
    Ok(())
}
