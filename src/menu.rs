use rg3d_core::pool::Handle;
use rg3d::{
    gui::{
        UserInterface,
        text_box::TextBoxBuilder,
        list_box::ListBoxBuilder,
        node::UINode,
        grid::{GridBuilder, Row, Column},
        Thickness,
        VerticalAlignment,
        window::{WindowBuilder, WindowTitle},
        text::TextBuilder,
        scroll_bar::ScrollBarBuilder,
        button::ButtonBuilder,
        Visibility,
        event::{UIEvent, UIEventKind},
        widget::{WidgetBuilder, AsWidget},
        HorizontalAlignment,
    },
    engine::{Engine, EngineInterfaceMut},
    event::WindowEvent,
    resource::ttf::Font,
    monitor::VideoMode,
    window::Fullscreen,
};
use std::{
    path::Path,
    rc::Rc,
    cell::RefCell,
};
use rg3d::gui::check_box::CheckBoxBuilder;

pub struct Menu {
    root: Handle<UINode>,
    options_window: Handle<UINode>,
    pub btn_new_game: Handle<UINode>,
    pub btn_save_game: Handle<UINode>,
    pub btn_load_game: Handle<UINode>,
    pub btn_quit_game: Handle<UINode>,
    pub sb_sound_volume: Handle<UINode>,
    pub sb_music_volume: Handle<UINode>,
    pub lb_video_modes: Handle<UINode>,
    cb_fullscreen: Handle<UINode>,
    video_modes: Vec<VideoMode>,
}

impl Menu {
    pub fn new(engine: &mut Engine) -> Self {
        let video_modes: Vec<VideoMode> = engine.get_window()
            .primary_monitor()
            .video_modes()
            .filter(|vm| vm.size().width > 800.0 &&
                vm.size().height > 600.0 && vm.bit_depth() == 32)
            .collect();

        let EngineInterfaceMut { ui, renderer, .. } = engine.interface_mut();

        let frame_size = renderer.get_frame_size();

        let margin = Thickness::uniform(2.0);

        let font: Font = Font::from_file(
            Path::new("data/ui/SquaresBold.ttf"),
            30.0,
            Font::default_char_set()).unwrap();
        let font = Rc::new(RefCell::new(font));

        let sb_sound_volume;
        let sb_music_volume;
        let lb_video_modes;
        let cb_fullscreen;
        let options_window: Handle<UINode> = WindowBuilder::new(WidgetBuilder::new()
            .with_width(400.0))
            .with_title(WindowTitle::Text("Options"))
            .open(false)
            .with_content(GridBuilder::new(WidgetBuilder::new()
                .with_margin(Thickness::uniform(5.0))
                .with_child(TextBuilder::new(WidgetBuilder::new()
                    .on_row(0)
                    .on_column(0)
                    .with_margin(margin))
                    .with_text("Sound Volume")
                    .with_vertical_text_alignment(VerticalAlignment::Center)
                    .build(ui))
                .with_child({
                    sb_sound_volume = ScrollBarBuilder::new(WidgetBuilder::new()
                        .on_row(0)
                        .on_column(1)
                        .with_margin(margin))
                        .with_min(0.0)
                        .with_max(1.0)
                        .with_value(1.0)
                        .with_step(0.025)
                        .build(ui);
                    sb_sound_volume
                })
                .with_child(TextBuilder::new(WidgetBuilder::new()
                    .on_row(1)
                    .on_column(0)
                    .with_margin(margin))
                    .with_text("Music Volume")
                    .with_vertical_text_alignment(VerticalAlignment::Center)
                    .build(ui))
                .with_child({
                    sb_music_volume = ScrollBarBuilder::new(WidgetBuilder::new()
                        .with_margin(margin)
                        .on_row(1)
                        .on_column(1))
                        .with_min(0.0)
                        .with_max(1.0)
                        .with_value(1.0)
                        .with_step(0.025)
                        .build(ui);
                    sb_music_volume
                })
                .with_child(TextBuilder::new(WidgetBuilder::new()
                    .on_row(2)
                    .on_column(0)
                    .with_margin(margin))
                    .with_text("Resolution")
                    .with_vertical_text_alignment(VerticalAlignment::Center)
                    .build(ui))
                .with_child({
                    lb_video_modes = ListBoxBuilder::new(WidgetBuilder::new()
                        .on_column(1)
                        .on_row(2))
                        .with_items({
                            let mut items = Vec::new();
                            for video_mode in video_modes.iter() {
                                let size = video_mode.size();
                                let rate = video_mode.refresh_rate();
                                let item = TextBuilder::new(WidgetBuilder::new()
                                    .on_column(0)
                                    .with_height(25.0)
                                    .with_width(200.0))
                                    .with_text(format!("{}x{}@{}Hz", size.width, size.height, rate).as_str())
                                    .with_vertical_text_alignment(VerticalAlignment::Center)
                                    .with_horizontal_text_alignment(HorizontalAlignment::Center)
                                    .build(ui);
                                items.push(item)
                            }
                            items
                        })
                        .build(ui);
                    lb_video_modes
                })
                .with_child(TextBuilder::new(WidgetBuilder::new()
                    .on_row(3)
                    .on_column(0)
                    .with_margin(margin))
                    .with_text("Player Name")
                    .with_vertical_text_alignment(VerticalAlignment::Center)
                    .build(ui))
                .with_child(TextBoxBuilder::new(WidgetBuilder::new()
                    .on_row(3)
                    .on_column(1))
                    .with_text("Unnamed Player".to_owned())
                    .build(ui))
                .with_child(TextBuilder::new(WidgetBuilder::new()
                    .on_row(4)
                    .on_column(0)
                    .with_margin(margin))
                    .with_text("Fullscreen")
                    .with_vertical_text_alignment(VerticalAlignment::Center)
                    .build(ui))
                .with_child({
                    cb_fullscreen = CheckBoxBuilder::new(WidgetBuilder::new()
                        .with_width(24.0)
                        .with_height(24.0)
                        .on_row(4)
                        .on_column(1)
                        .with_horizontal_alignment(HorizontalAlignment::Left))
                        .build(ui);
                    cb_fullscreen
                })
                .with_child(ButtonBuilder::new(WidgetBuilder::new()
                    .with_event_handler(Box::new(|ui, handle, evt| {
                        if evt.source() == handle {
                            match evt.kind {
                                UIEventKind::Click => {}
                                _ => ()
                            }
                        }
                    }))
                    .with_margin(Thickness::top(4.0))
                    .on_column(1)
                    .on_row(5))
                    .with_text("Apply")
                    .build(ui)))
                .add_row(Row::strict(34.0))
                .add_row(Row::strict(34.0))
                .add_row(Row::strict(200.0))
                .add_row(Row::strict(34.0))
                .add_row(Row::strict(34.0))
                .add_row(Row::strict(34.0))
                .add_column(Column::strict(150.0))
                .add_column(Column::stretch())
                .build(ui))
            .build(ui);

        let btn_new_game;
        let btn_save_game;
        let btn_load_game;
        let btn_quit_game;
        let root: Handle<UINode> = GridBuilder::new(WidgetBuilder::new()
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
                        ButtonBuilder::new(WidgetBuilder::new()
                            .with_event_handler(Box::new(move |ui, handle, evt| {
                                if evt.source() == handle {
                                    ui.send_event(UIEvent::targeted(options_window, UIEventKind::Opened));
                                }
                            }))
                            .on_column(0)
                            .on_row(3)
                            .with_margin(Thickness::uniform(4.0)))
                            .with_text("Settings")
                            .with_font(font.clone())
                            .build(ui)
                    })
                    .with_child({
                        btn_quit_game = ButtonBuilder::new(WidgetBuilder::new()
                            .on_column(0)
                            .on_row(4)
                            .with_margin(Thickness::uniform(4.0)))
                            .with_text("Quit")
                            .with_font(font.clone())
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
            root,
            options_window,
            btn_new_game,
            btn_save_game,
            btn_load_game,
            btn_quit_game,
            sb_sound_volume,
            sb_music_volume,
            lb_video_modes,
            cb_fullscreen,
            video_modes
        }
    }

    pub fn set_visible(&mut self, ui: &mut UserInterface, visible: bool) {
        let visibility = if visible { Visibility::Visible } else { Visibility::Collapsed };
        ui.get_node_mut(self.root).widget_mut().set_visibility(visibility);

        if !visible {
            ui.send_event(UIEvent::targeted(self.options_window, UIEventKind::Closed));
        }
    }

    pub fn is_visible(&self, ui: &UserInterface) -> bool {
        ui.get_node(self.root).widget().get_visibility() == Visibility::Visible
    }

    pub fn process_input_event(&mut self, engine: &mut Engine, event: &WindowEvent) {
        if let WindowEvent::Resized(new_size) = event {
            let EngineInterfaceMut { ui, renderer, .. } = engine.interface_mut();
            renderer.set_frame_size((*new_size).into()).unwrap();
            let root = ui.get_node_mut(self.root).widget_mut();
            root.set_width(new_size.width as f32);
            root.set_height(new_size.height as f32);
        }
    }

    pub fn process_ui_event(&mut self, engine: &mut Engine, event: &UIEvent) {
        let EngineInterfaceMut { sound_context, .. } = engine.interface_mut();

        match event.kind {
            UIEventKind::NumericValueChanged { new_value, .. } => {
                if event.source() == self.sb_sound_volume {
                    sound_context.lock().unwrap().set_master_gain(new_value)
                }
            }
            UIEventKind::SelectionChanged(new_value) => {
                if event.source() == self.lb_video_modes {
                    if let Some(index) = new_value {
                        let video_mode = self.video_modes[index].clone();
                        engine.get_window().set_fullscreen(Some(Fullscreen::Exclusive(video_mode)))
                    }
                }
            }
            _ => ()
        }
    }
}