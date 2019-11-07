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
    gui::check_box::CheckBoxBuilder,
};
use std::{
    path::Path,
    rc::Rc,
    cell::RefCell,
};

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
    cb_spot_shadows: Handle<UINode>,
    cb_soft_spot_shadows: Handle<UINode>,
    cb_point_shadows: Handle<UINode>,
    cb_soft_point_shadows: Handle<UINode>,
    sb_point_shadow_distance: Handle<UINode>,
    sb_spot_shadow_distance: Handle<UINode>,
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

        let settings = renderer.get_quality_settings();

        let sb_sound_volume;
        let sb_music_volume;
        let lb_video_modes;
        let cb_fullscreen;
        let cb_spot_shadows;
        let cb_soft_spot_shadows;
        let cb_point_shadows;
        let cb_soft_point_shadows;
        let sb_point_shadow_distance;
        let sb_spot_shadow_distance;
        let options_window: Handle<UINode> = WindowBuilder::new(WidgetBuilder::new()
            .with_width(500.0)
            .with_height(600.0))
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

                // Spot Shadows Enabled

                .with_child(TextBuilder::new(WidgetBuilder::new()
                    .on_row(5)
                    .on_column(0)
                    .with_margin(margin))
                    .with_text("Spot Shadows")
                    .with_vertical_text_alignment(VerticalAlignment::Center)
                    .build(ui))
                .with_child({
                    cb_spot_shadows = CheckBoxBuilder::new(WidgetBuilder::new()
                        .with_width(24.0)
                        .with_height(24.0)
                        .on_row(5)
                        .on_column(1)
                        .with_horizontal_alignment(HorizontalAlignment::Left))
                        .checked(Some(settings.spot_shadows_enabled))
                        .build(ui);
                    cb_spot_shadows
                })

                // Soft Spot Shadows

                .with_child(TextBuilder::new(WidgetBuilder::new()
                    .on_row(6)
                    .on_column(0)
                    .with_margin(margin))
                    .with_text("Soft Spot Shadows")
                    .with_vertical_text_alignment(VerticalAlignment::Center)
                    .build(ui))
                .with_child({
                    cb_soft_spot_shadows = CheckBoxBuilder::new(WidgetBuilder::new()
                        .with_width(24.0)
                        .with_height(24.0)
                        .on_row(6)
                        .on_column(1)
                        .with_horizontal_alignment(HorizontalAlignment::Left))
                        .checked(Some(settings.spot_soft_shadows))
                        .build(ui);
                    cb_soft_spot_shadows
                })

                // Spot Shadows Distance

                .with_child(TextBuilder::new(WidgetBuilder::new()
                    .on_row(7)
                    .on_column(0)
                    .with_margin(margin))
                    .with_text("Spot Shadows Distance")
                    .with_vertical_text_alignment(VerticalAlignment::Center)
                    .build(ui))
                .with_child({
                    sb_spot_shadow_distance = ScrollBarBuilder::new(WidgetBuilder::new()
                        .on_row(7)
                        .on_column(1)
                        .with_margin(margin))
                        .with_min(1.0)
                        .with_max(15.0)
                        .with_value(settings.spot_shadows_distance)
                        .with_step(0.25)
                        .build(ui);
                    sb_spot_shadow_distance
                })

                // Point Shadows Enabled

                .with_child(TextBuilder::new(WidgetBuilder::new()
                    .on_row(8)
                    .on_column(0)
                    .with_margin(margin))
                    .with_text("Point Shadows")
                    .with_vertical_text_alignment(VerticalAlignment::Center)
                    .build(ui))
                .with_child({
                    cb_point_shadows = CheckBoxBuilder::new(WidgetBuilder::new()
                        .with_width(24.0)
                        .with_height(24.0)
                        .on_row(8)
                        .on_column(1)
                        .with_horizontal_alignment(HorizontalAlignment::Left))
                        .checked(Some(settings.point_shadows_enabled))
                        .build(ui);
                    cb_point_shadows
                })

                // Soft Point Shadows

                .with_child(TextBuilder::new(WidgetBuilder::new()
                    .on_row(9)
                    .on_column(0)
                    .with_margin(margin))
                    .with_text("Soft Point Shadows")
                    .with_vertical_text_alignment(VerticalAlignment::Center)
                    .build(ui))
                .with_child({
                    cb_soft_point_shadows = CheckBoxBuilder::new(WidgetBuilder::new()
                        .with_width(24.0)
                        .with_height(24.0)
                        .on_row(9)
                        .on_column(1)
                        .with_horizontal_alignment(HorizontalAlignment::Left))
                        .checked(Some(settings.point_soft_shadows))
                        .build(ui);
                    cb_soft_point_shadows
                })

                // Point Shadows Distance

                .with_child(TextBuilder::new(WidgetBuilder::new()
                    .on_row(10)
                    .on_column(0)
                    .with_margin(margin))
                    .with_text("Point Shadows Distance")
                    .with_vertical_text_alignment(VerticalAlignment::Center)
                    .build(ui))
                .with_child({
                    sb_point_shadow_distance = ScrollBarBuilder::new(WidgetBuilder::new()
                        .on_row(10)
                        .on_column(1)
                        .with_margin(margin))
                        .with_min(1.0)
                        .with_max(15.0)
                        .with_value(settings.point_shadows_distance)
                        .with_step(0.25)
                        .build(ui);
                    sb_point_shadow_distance
                }))
                .add_row(Row::strict(34.0))
                .add_row(Row::strict(34.0))
                .add_row(Row::strict(200.0))
                .add_row(Row::strict(34.0))
                .add_row(Row::strict(34.0))
                .add_row(Row::strict(34.0))
                .add_row(Row::strict(34.0))
                .add_row(Row::strict(34.0))
                .add_row(Row::strict(34.0))
                .add_row(Row::strict(34.0))
                .add_row(Row::strict(34.0))
                .add_column(Column::strict(250.0))
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
            cb_spot_shadows,
            cb_soft_spot_shadows,
            cb_point_shadows,
            cb_soft_point_shadows,
            sb_point_shadow_distance,
            sb_spot_shadow_distance,
            video_modes,
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
        let EngineInterfaceMut { sound_context, renderer, .. } = engine.interface_mut();

        let old_settings = renderer.get_quality_settings();
        let mut settings = old_settings;

        match event.kind {
            UIEventKind::NumericValueChanged { new_value, .. } => {
                if event.source() == self.sb_sound_volume {
                    sound_context.lock().unwrap().set_master_gain(new_value)
                } else if event.source() == self.sb_point_shadow_distance {
                    settings.point_shadows_distance = new_value;
                } else if event.source() == self.sb_spot_shadow_distance {
                    settings.spot_shadows_distance = new_value;
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
            UIEventKind::Checked(value) => {
                if event.source() == self.cb_point_shadows {
                    settings.point_shadows_enabled = value.unwrap_or(false);
                } else if event.source() == self.cb_spot_shadows {
                    settings.spot_shadows_enabled = value.unwrap_or(false);
                } else if event.source() == self.cb_soft_spot_shadows {
                    settings.spot_soft_shadows = value.unwrap_or(false);
                } else if event.source() == self.cb_soft_spot_shadows {
                    settings.point_soft_shadows = value.unwrap_or(false);
                }
            }
            _ => ()
        }

        if settings != old_settings {
            if let Err(err) = engine.interface_mut().renderer.set_quality_settings(&settings) {
                println!("Failed to set quality settings! Reason: {:?}", err);
            }
        }
    }
}