mod libmpv_handlers;
mod music_player_core;
#[cfg_attr(
    not(target_os = "android"),
    path = "music_player/music_player_os_interface.rs"
)]
#[cfg_attr(
    target_os = "android",
    path = "music_player/music_player_os_interface_android.rs"
)]
mod music_player_os_interface;
mod tui;

pub struct MusicPlayer<'a> {
    libmpv: libmpv_handlers::LibMpvHandler,
    libmpv_signal_send: crossbeam::channel::Sender<libmpv_handlers::LibMpvSignals>,
    music_player_logic: music_player_core::MusicPlayerLogic<'a>,
    mp_logic_signal_send: crossbeam::channel::Sender<music_player_core::MusicPlayerLogicSignals>,
    tui: tui::MusicPlayerTUI,
    tui_signal_send: crossbeam::channel::Sender<tui::TuiSignals>,
    tui_input_handler: tui::user_input_handler::TUIUserInputHandler,
    music_player_os_interface: music_player_os_interface::MediaPlayerOSInterface,
    os_interface_signal_send:
        crossbeam::channel::Sender<music_player_os_interface::OSInterfaceSignals>,
}

impl<'a> MusicPlayer<'a> {
    pub fn new() -> Self {
        let config = std::fs::read_to_string("conf.json").unwrap_or_else(|_| {
            println!("Using default config");
            std::fs::read_to_string("def_conf.json").unwrap()
        });
        let config: music_player_core::MusicPlayerConfig = serde_json::from_str(&config).unwrap();

        let mut libmpv =
            libmpv_handlers::LibMpvHandler::initialize_libmpv(config.mpv_base_volume).unwrap();
        let libmpv_signal_send = libmpv.create_signal_channel();

        let mut music_player_tui = tui::MusicPlayerTUI::setup_terminal(config.mpv_base_volume);
        let tui_signal_send = music_player_tui.create_signal_channel();

        let tui_input_handler =
            tui::user_input_handler::TUIUserInputHandler::new(config.mpv_base_volume);

        let mut music_player_logic = music_player_core::MusicPlayerLogic::new(config);
        let mp_logic_signal_send = music_player_logic.create_signal_channel();

        let mut music_player_os_interface =
            music_player_os_interface::MediaPlayerOSInterface::new();
        let os_interface_signal_send = music_player_os_interface.create_signal_channel();

        MusicPlayer {
            libmpv,
            libmpv_signal_send,
            music_player_logic,
            mp_logic_signal_send,
            tui: music_player_tui,
            tui_signal_send,
            tui_input_handler,
            music_player_os_interface,
            os_interface_signal_send,
        }
    }

    pub fn play(&'a mut self, user_input: &str) {
        let ev_ctx = self.libmpv.create_event_context();
        let ev_ctx = ev_ctx.unwrap();

        let tui_input_handler_send = self.tui_input_handler.create_signal_channel();
        self.music_player_logic.set_signal_senders(
            &self.libmpv_signal_send,
            &self.os_interface_signal_send,
            &self.tui_signal_send,
            tui_input_handler_send,
        );

        crossbeam::scope(|scope| {
            scope.spawn(|_| self.libmpv.handle_signals());
            scope.spawn(|_| self.tui.handle_signals());
            scope.spawn(|_| {
                libmpv_handlers::libmpv_event_handling(
                    ev_ctx,
                    &self.mp_logic_signal_send,
                    &self.tui_signal_send,
                )
            });
            scope.spawn(|_| {
                self.music_player_logic.process_user_input(user_input);
                self.music_player_logic.handle_playback_logic();
            });
            scope.spawn(|_| {
                self.tui_input_handler.handle_user_input(
                    &self.libmpv_signal_send,
                    &self.tui_signal_send,
                    &self.mp_logic_signal_send,
                )
            });
            scope.spawn(|_| {
                self.music_player_os_interface
                    .handle_signals(&self.libmpv_signal_send, &self.mp_logic_signal_send)
            });
        })
        .unwrap();

        self.tui.restore_terminal();
    }
}
