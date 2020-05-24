use std::{
    path::Path,
    rc::Rc,
    sync::{
        Mutex,
        Arc,
        mpsc::Sender,
    },
    cell::RefCell,
};
use crate::{
    message::Message,
    match_menu::MatchMenu,
    options_menu::OptionsMenu,
    UINodeHandle,
    GameEngine,
    Gui,
    GuiMessage,
    control_scheme::ControlScheme,
};
use rg3d::{
    core::math::vec2::Vec2,
    event::{
        WindowEvent,
        Event,
    },
    gui::{
        ttf::Font,
        grid::{
            GridBuilder,
            Row,
            Column,
        },
        Thickness,
        window::{
            WindowBuilder,
            WindowTitle,
        },
        button::ButtonBuilder,
        message::{
            UiMessage,
            UiMessageData,
            WindowMessage,
            ButtonMessage,
            WidgetMessage,
            WidgetProperty,
        },
        widget::WidgetBuilder,
    },
};

pub struct Menu {
    sender: Sender<Message>,
    root: UINodeHandle,
    btn_new_game: UINodeHandle,
    btn_save_game: UINodeHandle,
    btn_settings: UINodeHandle,
    btn_load_game: UINodeHandle,
    btn_quit_game: UINodeHandle,
    options_menu: OptionsMenu,
    match_menu: MatchMenu,
}

impl Menu {
    pub fn new(engine: &mut GameEngine, control_scheme: Rc<RefCell<ControlScheme>>, sender: Sender<Message>) -> Self {
        let frame_size = engine.renderer.get_frame_size();

        let font: Font = Font::from_file(
            Path::new("data/ui/SquaresBold.ttf"),
            30.0,
            Font::default_char_set()).unwrap();
        let font = Arc::new(Mutex::new(font));

        let ui = &mut engine.user_interface;

        let btn_new_game;
        let btn_settings;
        let btn_save_game;
        let btn_load_game;
        let btn_quit_game;
        let root: UINodeHandle = GridBuilder::new(WidgetBuilder::new()
            .with_width(frame_size.0 as f32)
            .with_height(frame_size.1 as f32)
            .with_child(WindowBuilder::new(WidgetBuilder::new()
                .on_row(1)
                .on_column(1))
                .can_minimize(false)
                .can_close(false)
                .with_title(WindowTitle::Text("Rusty Shooter"))
                .with_content(GridBuilder::new(WidgetBuilder::new()
                    .with_margin(Thickness::uniform(20.0))
                    .with_child({
                        btn_new_game = ButtonBuilder::new(WidgetBuilder::new()
                            .on_column(0)
                            .on_row(0)
                            .with_margin(Thickness::uniform(4.0)))
                            .with_text("New Game")
                            .with_font(font.clone())
                            .build(ui);
                        btn_new_game
                    })
                    .with_child({
                        btn_save_game = ButtonBuilder::new(WidgetBuilder::new()
                            .on_column(0)
                            .on_row(1)
                            .with_margin(Thickness::uniform(4.0)))
                            .with_text("Save Game")
                            .with_font(font.clone())
                            .build(ui);
                        btn_save_game
                    })
                    .with_child({
                        btn_load_game = ButtonBuilder::new(WidgetBuilder::new()
                            .on_column(0)
                            .on_row(2)
                            .with_margin(Thickness::uniform(4.0)))
                            .with_text("Load Game")
                            .with_font(font.clone())
                            .build(ui);
                        btn_load_game
                    })
                    .with_child({
                        btn_settings = ButtonBuilder::new(WidgetBuilder::new()
                            .on_column(0)
                            .on_row(3)
                            .with_margin(Thickness::uniform(4.0)))
                            .with_text("Settings")
                            .with_font(font.clone())
                            .build(ui);
                        btn_settings
                    })
                    .with_child({
                        btn_quit_game = ButtonBuilder::new(WidgetBuilder::new()
                            .on_column(0)
                            .on_row(4)
                            .with_margin(Thickness::uniform(4.0)))
                            .with_text("Quit")
                            .with_font(font)
                            .build(ui);
                        btn_quit_game
                    }))
                    .add_column(Column::stretch())
                    .add_row(Row::strict(75.0))
                    .add_row(Row::strict(75.0))
                    .add_row(Row::strict(75.0))
                    .add_row(Row::strict(75.0))
                    .add_row(Row::strict(75.0))
                    .build(ui))
                .build(ui)))
            .add_row(Row::stretch())
            .add_row(Row::strict(500.0))
            .add_row(Row::stretch())
            .add_column(Column::stretch())
            .add_column(Column::strict(400.0))
            .add_column(Column::stretch())
            .build(ui);

        Self {
            sender: sender.clone(),
            root,
            btn_new_game,
            btn_settings,
            btn_save_game,
            btn_load_game,
            btn_quit_game,
            options_menu: OptionsMenu::new(engine, control_scheme, sender.clone()),
            match_menu: MatchMenu::new(&mut engine.user_interface, &mut engine.resource_manager.lock().unwrap(), sender),
        }
    }

    pub fn set_visible(&mut self, ui: &mut Gui, visible: bool) {
        ui.node_mut(self.root).set_visibility(visible);

        if !visible {
            ui.send_message(UiMessage {
                destination: self.options_menu.window,
                data: UiMessageData::Window(WindowMessage::Close),
                handled: false,
            });
            ui.send_message(UiMessage {
                destination: self.match_menu.window,
                data: UiMessageData::Window(WindowMessage::Close),
                handled: false,
            });
        }
    }

    pub fn is_visible(&self, ui: &Gui) -> bool {
        ui.node(self.root).visibility()
    }

    pub fn process_input_event(&mut self, engine: &mut GameEngine, event: &Event<()>) {
        if let Event::WindowEvent { event, .. } = event {
            if let WindowEvent::Resized(new_size) = event {
                engine.user_interface
                    .node_mut(self.root)
                    .set_width_mut(new_size.width as f32)
                    .set_height_mut(new_size.height as f32);
            }
        }

        self.options_menu.process_input_event(engine, event);
    }

    pub fn handle_ui_event(&mut self, engine: &mut GameEngine, message: &GuiMessage) {
        if let UiMessageData::Button(msg) = &message.data {
            if let ButtonMessage::Click = msg {
                if message.destination == self.btn_new_game {
                    engine.user_interface
                        .send_message(UiMessage {
                            destination: self.match_menu.window,
                            data: UiMessageData::Window(WindowMessage::Open),
                            handled: false,
                        });
                    engine.user_interface
                        .send_message(UiMessage {
                            destination: self.match_menu.window,
                            data: UiMessageData::Widget(WidgetMessage::Property(
                                WidgetProperty::DesiredPosition(Vec2::new(400.0, 0.0)))),
                            handled: false,
                        })
                } else if message.destination == self.btn_save_game {
                    self.sender
                        .send(Message::SaveGame)
                        .unwrap();
                } else if message.destination == self.btn_load_game {
                    self.sender
                        .send(Message::LoadGame)
                        .unwrap();
                } else if message.destination == self.btn_quit_game {
                    self.sender
                        .send(Message::QuitGame)
                        .unwrap();
                } else if message.destination == self.btn_settings {
                    engine.user_interface
                        .send_message(UiMessage {
                            destination: self.options_menu.window,
                            data: UiMessageData::Window(WindowMessage::Open),
                            handled: false
                        });
                    engine.user_interface
                        .send_message(UiMessage {
                            destination: self.options_menu.window,
                            data: UiMessageData::Widget(
                                WidgetMessage::Property(
                                    WidgetProperty::DesiredPosition(Vec2::new(200.0, 200.0)))),
                            handled: false
                        })
                }
            }
        }

        self.options_menu.handle_ui_event(engine, message);
        self.match_menu.handle_ui_event(engine, message);
    }
}