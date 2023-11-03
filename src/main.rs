fn main() {
    let mut args: Vec<String> = std::env::args().collect();
    let user_input: Option<String> = args.pop();

    let args = &args[1..];
    let mut music_player = rustunes::music_player::MusicPlayer::new(args);
    if let Some(user_input) = user_input {
        music_player.play(&user_input);
    } else {
        println!("No input given");
    }
}
