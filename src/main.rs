use rustunes::music_player::{
    logger::LogSender, music_player_config::MusicPlayerOptions, MusicPlayer,
};

fn main() {
    let mut args: Vec<String> = std::env::args().collect();
    let user_input: Option<String> = {
        if args.len() > 1 {
            args.pop()
        } else {
            None
        }
    };

    if let Some(user_input) = user_input {
        let args = &args[1..];
        let mut music_player = MusicPlayer::new(args);
        music_player.play(&user_input);
    } else {
        let log_send = LogSender::new(None);
        MusicPlayerOptions::new(log_send.clone()).print_help();
    }
}
