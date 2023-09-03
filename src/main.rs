fn main() {
    let args: Vec<String> = std::env::args().collect();
    let user_input: Option<&String> = args.get(1);

    let config = std::fs::read_to_string("conf.json").unwrap_or_else(|_| {
        println!("Using default config");
        std::fs::read_to_string("def_conf.json").unwrap()
    });
    let config = serde_json::from_str(&config).unwrap();

    let mut music_player = rustunes::music_player_core::MusicPlayer::new_from_config(config);

    if let Some(user_input) = user_input {
        music_player.play(user_input);
    } else {
        println!("No input given");
    }
}
