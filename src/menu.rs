use crate::{
    control_scheme::ControlScheme, match_menu::MatchMenu, message::Message,
    options_menu::OptionsMenu,
};
use rg3d::{
    core::pool::Handle,
    engine::Engine,
    event::{Event, WindowEvent},
    gui::{
        button::{ButtonBuilder, ButtonMessage},
        grid::{Column, GridBuilder, Row},
        message::{MessageDirection, UiMessage},
        ttf::{Font, SharedFont},
        widget::{WidgetBuilder, WidgetMessage},
        window::{WindowBuilder, WindowMessage, WindowTitle},
        Thickness, UiNode, UserInterface,
    },
};
use std::{
    path::Path,
    sync::{mpsc::Sender, Arc, Mutex, RwLock},
};

pub struct Menu {
    sender: Sender<Message>,
    root: Handle<UiNode>,
    btn_new_game: Handle<UiNode>,
    btn_save_game: Handle<UiNode>,
    btn_settings: Handle<UiNode>,
    btn_load_game: Handle<UiNode>,
    btn_quit_game: Handle<UiNode>,
    options_menu: OptionsMenu,
    match_menu: MatchMenu,
}

impl Menu {
    pub fn new(
        engine: &mut Engine,
        control_scheme: Arc<RwLock<ControlScheme>>,
        sender: Sender<Message>,
    ) -> Self {
        let frame_size = engine.renderer.get_frame_size();

        let font: Font = rg3d::core::futures::executor::block_on(Font::from_file(
            Path::new("data/ui/SquaresBold.ttf"),
            31.0,
            Font::default_char_set(),
        ))
        .unwrap();
        let font = SharedFont(Arc::new(Mutex::new(font)));

        let ctx = &mut engine.user_interface.build_ctx();

        let btn_new_game;
        let btn_settings;
        let btn_save_game;
        let btn_load_game;
        let btn_quit_game;
        let root: Handle<UiNode> = GridBuilder::new(
            WidgetBuilder::new()
                .with_width(frame_size.0 as f32)
                .with_height(frame_size.1 as f32)
                .with_child(
                    WindowBuilder::new(WidgetBuilder::new().on_row(1).on_column(1))
                        .can_resize(false)
                        .can_minimize(false)
                        .can_close(false)
                        .with_title(WindowTitle::text("Rusty Shooter"))
                        .with_content(
                            GridBuilder::new(
                                WidgetBuilder::new()
                                    .with_margin(Thickness::uniform(20.0))
                                    .with_child({
                                        btn_new_game = ButtonBuilder::new(
                                            WidgetBuilder::new()
                                                .on_column(0)
                                                .on_row(0)
                                                .with_margin(Thickness::uniform(4.0)),
                                        )
                                        .with_text("New Game")
                                        .with_font(font.clone())
                                        .build(ctx);
                                        btn_new_game
                                    })
                                    .with_child({
                                        btn_save_game = ButtonBuilder::new(
                                            WidgetBuilder::new()
                                                .on_column(0)
                                                .on_row(1)
                                                .with_margin(Thickness::uniform(4.0)),
                                        )
                                        .with_text("Save Game")
                                        .with_font(font.clone())
                                        .build(ctx);
                                        btn_save_game
                                    })
                                    .with_child({
                                        btn_load_game = ButtonBuilder::new(
                                            WidgetBuilder::new()
                                                .on_column(0)
                                                .on_row(2)
                                                .with_margin(Thickness::uniform(4.0)),
                                        )
                                        .with_text("Load Game")
                                        .with_font(font.clone())
                                        .build(ctx);
                                        btn_load_game
                                    })
                                    .with_child({
                                        btn_settings = ButtonBuilder::new(
                                            WidgetBuilder::new()
                                                .on_column(0)
                                                .on_row(3)
                                                .with_margin(Thickness::uniform(4.0)),
                                        )
                                        .with_text("Settings")
                                        .with_font(font.clone())
                                        .build(ctx);
                                        btn_settings
                                    })
                                    .with_child({
                                        btn_quit_game = ButtonBuilder::new(
                                            WidgetBuilder::new()
                                                .on_column(0)
                                                .on_row(4)
                                                .with_margin(Thickness::uniform(4.0)),
                                        )
                                        .with_text("Quit")
                                        .with_font(font)
                                        .build(ctx);
                                        btn_quit_game
                                    }),
                            )
                            .add_column(Column::stretch())
                            .add_row(Row::strict(75.0))
                            .add_row(Row::strict(75.0))
                            .add_row(Row::strict(75.0))
                            .add_row(Row::strict(75.0))
                            .add_row(Row::strict(75.0))
                            .build(ctx),
                        )
                        .build(ctx),
                ),
        )
        .add_row(Row::stretch())
        .add_row(Row::strict(500.0))
        .add_row(Row::stretch())
        .add_column(Column::stretch())
        .add_column(Column::strict(400.0))
        .add_column(Column::stretch())
        .build(ctx);

        Self {
            sender: sender.clone(),
            root,
            btn_new_game,
            btn_settings,
            btn_save_game,
            btn_load_game,
            btn_quit_game,
            options_menu: OptionsMenu::new(engine, control_scheme, sender.clone()),
            match_menu: MatchMenu::new(&mut engine.user_interface, sender),
        }
    }

    pub fn set_visible(&mut self, ui: &mut UserInterface, visible: bool) {
        ui.send_message(WidgetMessage::visibility(
            self.root,
            MessageDirection::ToWidget,
            visible,
        ));
        if !visible {
            ui.send_message(WindowMessage::close(
                self.options_menu.window,
                MessageDirection::ToWidget,
            ));
            ui.send_message(WindowMessage::close(
                self.match_menu.window,
                MessageDirection::ToWidget,
            ));
        }
    }

    pub fn is_visible(&self, ui: &UserInterface) -> bool {
        ui.node(self.root).visibility()
    }

    pub fn process_input_event(&mut self, engine: &mut Engine, event: &Event<()>) {
        if let Event::WindowEvent { event, .. } = event {
            if let WindowEvent::Resized(new_size) = event {
                engine.user_interface.send_message(WidgetMessage::width(
                    self.root,
                    MessageDirection::ToWidget,
                    new_size.width as f32,
                ));
                engine.user_interface.send_message(WidgetMessage::height(
                    self.root,
                    MessageDirection::ToWidget,
                    new_size.height as f32,
                ));
            }
        }

        self.options_menu.process_input_event(engine, event);
    }

    pub fn handle_ui_event(&mut self, engine: &mut Engine, message: &UiMessage) {
        if let Some(ButtonMessage::Click) = message.data() {
            if message.destination() == self.btn_new_game {
                engine.user_interface.send_message(WindowMessage::open(
                    self.match_menu.window,
                    MessageDirection::ToWidget,
                    true,
                ));
            } else if message.destination() == self.btn_save_game {
                self.sender.send(Message::SaveGame).unwrap();
            } else if message.destination() == self.btn_load_game {
                self.sender.send(Message::LoadGame).unwrap();
            } else if message.destination() == self.btn_quit_game {
                self.sender.send(Message::QuitGame).unwrap();
            } else if message.destination() == self.btn_settings {
                engine.user_interface.send_message(WindowMessage::open(
                    self.options_menu.window,
                    MessageDirection::ToWidget,
                    true,
                ));
            }
        }

        self.options_menu.handle_ui_event(engine, message);
        self.match_menu.handle_ui_event(engine, message);
    }
}
