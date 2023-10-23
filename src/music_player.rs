mod libmpv_handlers;
mod logger;
mod music_player_config;
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

pub struct MusicPlayer {
    libmpv: libmpv_handlers::LibMpvHandler,
    libmpv_event_handler: libmpv_handlers::EventHandler,
    music_player_logic: music_player_core::MusicPlayerLogic,
    tui: tui::MusicPlayerTUI,
    tui_input_handler: tui::user_input_handler::TUIUserInputHandler,
    music_player_os_interface: music_player_os_interface::MediaPlayerOSInterface,
    logger: Option<logger::Logger>,
}

impl MusicPlayer {
    pub fn new() -> Self {
        let config: music_player_config::MusicPlayerConfig =
            music_player_config::MusicPlayerConfig::new().unwrap();
        let logger = {
            if config.debug_log {
                Some(logger::Logger::new())
            } else {
                None
            }
        };
        let log_send = {
            if config.debug_log {
                logger::LogSender::new(Some(logger.as_ref().unwrap().get_signal_send()))
            } else {
                logger::LogSender::new(None)
            }
        };

        let mut libmpv = libmpv_handlers::LibMpvHandler::initialize_libmpv(
            config.mpv_base_volume,
            log_send.clone(),
        )
        .unwrap();
        let libmpv_signal_send = libmpv.create_signal_channel();

        let mut music_player_tui =
            tui::MusicPlayerTUI::setup_terminal(config.mpv_base_volume, log_send.clone());
        let tui_signal_send = music_player_tui.create_signal_channel();

        let mut tui_input_handler = tui::user_input_handler::TUIUserInputHandler::new(
            config.mpv_base_volume,
            log_send.clone(),
        );
        let tui_input_handler_send = tui_input_handler.create_signal_channel();

        let mut music_player_logic =
            music_player_core::MusicPlayerLogic::new(config, log_send.clone());
        let mp_logic_signal_send = music_player_logic.create_signal_channel();

        let mut music_player_os_interface =
            music_player_os_interface::MediaPlayerOSInterface::new(log_send.clone());
        let os_interface_signal_send = music_player_os_interface.create_signal_channel();

        let libmpv_event_handler = libmpv_handlers::EventHandler::new(
            mp_logic_signal_send.clone(),
            tui_signal_send.clone(),
            log_send,
        );

        music_player_logic.set_signal_senders(
            libmpv_signal_send.clone(),
            os_interface_signal_send,
            tui_signal_send.clone(),
            tui_input_handler_send,
        );

        tui_input_handler.set_senders(
            libmpv_signal_send.clone(),
            tui_signal_send,
            mp_logic_signal_send.clone(),
        );

        music_player_os_interface.set_senders(libmpv_signal_send, mp_logic_signal_send);

        MusicPlayer {
            libmpv,
            libmpv_event_handler,
            music_player_logic,
            tui: music_player_tui,
            tui_input_handler,
            music_player_os_interface,
            logger,
        }
    }

    pub fn play(&mut self, user_input: &str) {
        let ev_ctx = self.libmpv.create_event_context();
        let ev_ctx = ev_ctx.unwrap();

        let mut error: Result<(), logger::Error> = Ok(());
        crossbeam::scope(|scope| {
            scope.spawn(|_| self.libmpv.handle_signals());
            scope.spawn(|_| self.tui.handle_signals());
            scope.spawn(|_| self.libmpv_event_handler.libmpv_event_handling(ev_ctx));
            scope.spawn(|_| {
                error = self.music_player_logic.process_user_input(user_input);
                if error.is_ok() {
                    error = self.music_player_logic.handle_playback_logic();
                }
            });
            scope.spawn(|_| self.tui_input_handler.handle_user_input());
            scope.spawn(|_| self.music_player_os_interface.handle_signals());

            if let Some(logger) = &self.logger {
                scope.spawn(|_| logger.log());
            }
        })
        .unwrap();

        self.tui.restore_terminal();

        error.unwrap();
    }
}
