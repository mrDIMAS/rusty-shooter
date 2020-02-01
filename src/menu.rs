use std::{
    path::Path,
    rc::Rc,
    sync::{Mutex, Arc},
    cell::RefCell,
};
use crate::{
    control_scheme::{
        ControlScheme,
        ControlButton,
    }
};
use rg3d::{
    resource::{
        texture::TextureKind,
    },
    engine::Engine, event::{
        WindowEvent, Event, MouseScrollDelta,
        MouseButton
    },
    monitor::VideoMode, window::Fullscreen,
    core::{
        pool::Handle,
    },
    gui::{
        ttf::Font,
        check_box::CheckBox,
        UINodeContainer,
        ControlTemplate,
        style::StyleBuilder,
        check_box::CheckBoxBuilder,
        UserInterface,
        text_box::TextBoxBuilder,
        list_box::ListBoxBuilder,
        UINode,
        grid::{
            GridBuilder,
            Row,
            Column,
        },
        Thickness,
        VerticalAlignment,
        window::{
            WindowBuilder,
            WindowTitle,
        },
        text::{
            TextBuilder,
            Text,
        },
        scroll_bar::{
            ScrollBarBuilder,
            ScrollBar,
        },
        button::{
            ButtonBuilder,
            Button,
        },
        Visibility,
        event::{
            UIEvent,
            UIEventKind,
        },
        widget::{
            WidgetBuilder,
            Widget,
        },
        HorizontalAlignment,
        Control,
        Builder,
        image::ImageBuilder,
        tab_control::{
            TabControlBuilder,
            TabDefinition,
        },
    },
    utils,
};

pub struct Menu {
    root: Handle<UINode>,
    options_window: Handle<UINode>,
    pub btn_new_game: Handle<UINode>,
    pub btn_save_game: Handle<UINode>,
    pub btn_settings: Handle<UINode>,
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
    control_scheme: Rc<RefCell<ControlScheme>>,
    control_scheme_buttons: Vec<Handle<UINode>>,
    active_control_button: Option<usize>,
    sb_mouse_sens: Handle<UINode>,
    cb_mouse_y_inverse: Handle<UINode>,
    cb_smooth_mouse: Handle<UINode>,
    cb_shake_camera: Handle<UINode>,
    btn_reset_control_scheme: Handle<UINode>,
    cb_use_hrtf: Handle<UINode>,
    btn_reset_audio_settings: Handle<UINode>,
}

impl Menu {
    pub fn new(engine: &mut Engine, control_scheme: Rc<RefCell<ControlScheme>>) -> Self {
        let video_modes: Vec<VideoMode> = engine.get_window()
            .primary_monitor()
            .video_modes()
            .filter(|vm| vm.size().width > 800 &&
                vm.size().height > 600 && vm.bit_depth() == 32)
            .collect();

        let frame_size = engine.renderer.get_frame_size();

        let font: Font = Font::from_file(
            Path::new("data/ui/SquaresBold.ttf"),
            30.0,
            Font::default_char_set()).unwrap();
        let font = Arc::new(Mutex::new(font));

        let settings = engine.renderer.get_quality_settings();

        let common_style = Rc::new(StyleBuilder::new()
            .with_setter(Widget::MARGIN, Box::new(Thickness::uniform(2.0)))
            .build());

        let mut check_box_template = ControlTemplate::new();
        CheckBoxBuilder::new(WidgetBuilder::new()
            .with_style(common_style.clone())
            .with_width(24.0)
            .with_height(24.0)
            .with_vertical_alignment(VerticalAlignment::Center)
            .with_horizontal_alignment(HorizontalAlignment::Left))
            .with_check_mark(ImageBuilder::new(WidgetBuilder::new())
                .with_opt_texture(utils::into_any_arc(engine.resource_manager.request_texture("data/ui/check_mark.png", TextureKind::RGBA8)))
                .build(&mut check_box_template))
            .build(&mut check_box_template);

        let mut scroll_bar_template = ControlTemplate::new();
        ScrollBarBuilder::new(WidgetBuilder::new()
            .with_style(common_style.clone()))
            .with_indicator(ImageBuilder::new(WidgetBuilder::new())
                .with_opt_texture(utils::into_any_arc(engine.resource_manager.request_texture("data/ui/circle.png", TextureKind::RGBA8)))
                .build(&mut scroll_bar_template))
            .build(&mut scroll_bar_template);

        let common_row = Row::strict(36.0);
        let ui = &mut engine.user_interface;

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
        let sb_mouse_sens;
        let cb_mouse_y_inverse;
        let cb_smooth_mouse;
        let cb_shake_camera;
        let btn_reset_control_scheme;
        let mut control_scheme_buttons = Vec::new();
        let cb_use_hrtf;
        let btn_reset_audio_settings;
        let tab_control = TabControlBuilder::new(WidgetBuilder::new())
            .with_tab(TabDefinition {
                header: {
                    TextBuilder::new(WidgetBuilder::new()
                        .with_width(100.0)
                        .with_height(30.0))
                        .with_text("Graphics")
                        .build(ui)
                },
                content: {
                    GridBuilder::new(WidgetBuilder::new()
                        .with_margin(Thickness::uniform(5.0))
                        .with_child(TextBuilder::new(WidgetBuilder::new()
                            .on_row(0)
                            .on_column(0)
                            .with_style(common_style.clone()))
                            .with_text("Resolution")
                            .with_vertical_text_alignment(VerticalAlignment::Center)
                            .build(ui))
                        .with_child({
                            lb_video_modes = ListBoxBuilder::new(WidgetBuilder::new()
                                .on_column(1)
                                .on_row(0)
                                .with_style(common_style.clone()))
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
                            .on_row(1)
                            .on_column(0)
                            .with_style(common_style.clone()))
                            .with_text("Player Name")
                            .with_vertical_text_alignment(VerticalAlignment::Center)
                            .build(ui))
                        .with_child(TextBoxBuilder::new(WidgetBuilder::new()
                            .on_row(1)
                            .on_column(1)
                            .with_style(common_style.clone()))
                            .with_text("Unnamed Player".to_owned())
                            .build(ui))
                        .with_child(TextBuilder::new(WidgetBuilder::new()
                            .on_row(2)
                            .on_column(0)
                            .with_style(common_style.clone()))
                            .with_text("Fullscreen")
                            .with_vertical_text_alignment(VerticalAlignment::Center)
                            .build(ui))
                        .with_child({
                            cb_fullscreen = check_box_template.instantiate(ui);
                            ui.node_mut(cb_fullscreen)
                                .downcast_mut::<CheckBox>()
                                .unwrap()
                                .set_checked(Some(false))
                                .widget_mut()
                                .set_row(2)
                                .set_column(1);
                            cb_fullscreen
                        })

                        // Spot Shadows Enabled

                        .with_child(TextBuilder::new(WidgetBuilder::new()
                            .on_row(3)
                            .on_column(0)
                            .with_style(common_style.clone()))
                            .with_text("Spot Shadows")
                            .with_vertical_text_alignment(VerticalAlignment::Center)
                            .build(ui))
                        .with_child({
                            cb_spot_shadows = check_box_template.instantiate(ui);
                            ui.node_mut(cb_spot_shadows)
                                .downcast_mut::<CheckBox>()
                                .unwrap()
                                .set_checked(Some(settings.spot_shadows_enabled))
                                .widget_mut()
                                .set_row(3)
                                .set_column(1);
                            cb_spot_shadows
                        })

                        // Soft Spot Shadows

                        .with_child(TextBuilder::new(WidgetBuilder::new()
                            .on_row(4)
                            .on_column(0)
                            .with_style(common_style.clone()))
                            .with_text("Soft Spot Shadows")
                            .with_vertical_text_alignment(VerticalAlignment::Center)
                            .build(ui))
                        .with_child({
                            cb_soft_spot_shadows = check_box_template.instantiate(ui);
                            ui.node_mut(cb_soft_spot_shadows)
                                .downcast_mut::<CheckBox>()
                                .unwrap()
                                .set_checked(Some(settings.spot_soft_shadows))
                                .widget_mut()
                                .set_row(4)
                                .set_column(1);
                            cb_soft_spot_shadows
                        })

                        // Spot Shadows Distance

                        .with_child(TextBuilder::new(WidgetBuilder::new()
                            .on_row(5)
                            .on_column(0)
                            .with_style(common_style.clone()))
                            .with_text("Spot Shadows Distance")
                            .with_vertical_text_alignment(VerticalAlignment::Center)
                            .build(ui))
                        .with_child({
                            sb_spot_shadow_distance = scroll_bar_template.instantiate(ui);
                            ui.node_mut(sb_spot_shadow_distance)
                                .downcast_mut::<ScrollBar>()
                                .unwrap()
                                .set_min_value(1.0)
                                .set_max_value(15.0)
                                .set_value(settings.spot_shadows_distance)
                                .set_step(0.25)
                                .widget_mut()
                                .set_row(5)
                                .set_column(1);
                            sb_spot_shadow_distance
                        })

                        // Point Shadows Enabled

                        .with_child(TextBuilder::new(WidgetBuilder::new()
                            .on_row(6)
                            .on_column(0)
                            .with_style(common_style.clone()))
                            .with_text("Point Shadows")
                            .with_vertical_text_alignment(VerticalAlignment::Center)
                            .build(ui))
                        .with_child({
                            cb_point_shadows = check_box_template.instantiate(ui);
                            ui.node_mut(cb_point_shadows)
                                .downcast_mut::<CheckBox>()
                                .unwrap()
                                .set_checked(Some(settings.point_shadows_enabled))
                                .widget_mut()
                                .set_row(6)
                                .set_column(1);
                            cb_point_shadows
                        })

                        // Soft Point Shadows

                        .with_child(TextBuilder::new(WidgetBuilder::new()
                            .on_row(7)
                            .on_column(0)
                            .with_style(common_style.clone()))
                            .with_text("Soft Point Shadows")
                            .with_vertical_text_alignment(VerticalAlignment::Center)
                            .build(ui))
                        .with_child({
                            cb_soft_point_shadows = check_box_template.instantiate(ui);
                            ui.node_mut(cb_soft_point_shadows)
                                .downcast_mut::<CheckBox>()
                                .unwrap()
                                .set_checked(Some(settings.point_soft_shadows))
                                .widget_mut()
                                .set_row(7)
                                .set_column(1);
                            cb_soft_point_shadows
                        })

                        // Point Shadows Distance

                        .with_child(TextBuilder::new(WidgetBuilder::new()
                            .on_row(8)
                            .on_column(0)
                            .with_style(common_style.clone()))
                            .with_text("Point Shadows Distance")
                            .with_vertical_text_alignment(VerticalAlignment::Center)
                            .build(ui))
                        .with_child({
                            sb_point_shadow_distance = scroll_bar_template.instantiate(ui);
                            ui.node_mut(sb_point_shadow_distance)
                                .downcast_mut::<ScrollBar>()
                                .unwrap()
                                .set_min_value(1.0)
                                .set_max_value(15.0)
                                .set_value(settings.point_shadows_distance)
                                .set_step(0.25)
                                .widget_mut()
                                .set_row(8)
                                .set_column(1);
                            sb_point_shadow_distance
                        }))
                        .add_row(Row::strict(200.0))
                        .add_row(common_row)
                        .add_row(common_row)
                        .add_row(common_row)
                        .add_row(common_row)
                        .add_row(common_row)
                        .add_row(common_row)
                        .add_row(common_row)
                        .add_row(common_row)
                        .add_column(Column::strict(250.0))
                        .add_column(Column::stretch())
                        .build(ui)
                },
            })
            .with_tab(TabDefinition {
                header: {
                    TextBuilder::new(WidgetBuilder::new()
                        .with_width(100.0)
                        .with_height(30.0))
                        .with_text("Sound")
                        .build(ui)
                },
                content: {
                    GridBuilder::new(WidgetBuilder::new()
                        .with_child(TextBuilder::new(WidgetBuilder::new()
                            .on_row(0)
                            .on_column(0)
                            .with_style(common_style.clone()))
                            .with_text("Sound Volume")
                            .with_vertical_text_alignment(VerticalAlignment::Center)
                            .build(ui))
                        .with_child({
                            sb_sound_volume = scroll_bar_template.instantiate(ui);
                            ui.node_mut(sb_sound_volume)
                                .downcast_mut::<ScrollBar>()
                                .unwrap()
                                .set_min_value(0.0)
                                .set_max_value(1.0)
                                .set_value(1.0)
                                .set_step(0.025)
                                .widget_mut()
                                .set_row(0)
                                .set_column(1);
                            sb_sound_volume
                        })
                        .with_child(TextBuilder::new(WidgetBuilder::new()
                            .on_row(1)
                            .on_column(0)
                            .with_style(common_style.clone()))
                            .with_text("Music Volume")
                            .with_vertical_text_alignment(VerticalAlignment::Center)
                            .build(ui))
                        .with_child({
                            sb_music_volume = scroll_bar_template.instantiate(ui);
                            ui.node_mut(sb_music_volume)
                                .downcast_mut::<ScrollBar>()
                                .unwrap()
                                .set_min_value(0.0)
                                .set_max_value(1.0)
                                .set_value(0.0)
                                .set_step(0.025)
                                .widget_mut()
                                .set_row(1)
                                .set_column(1);
                            sb_music_volume
                        })
                        .with_child(TextBuilder::new(WidgetBuilder::new()
                            .on_row(2)
                            .on_column(0)
                            .with_style(common_style.clone()))
                            .with_text("Use HRTF")
                            .with_vertical_text_alignment(VerticalAlignment::Center)
                            .build(ui))
                        .with_child({
                            cb_use_hrtf = check_box_template.instantiate(ui);
                            ui.node_mut(cb_use_hrtf)
                                .downcast_mut::<CheckBox>()
                                .unwrap()
                                .set_checked(Some(true))
                                .widget_mut()
                                .set_row(2)
                                .set_column(1);
                            cb_use_hrtf
                        })
                        .with_child({
                            btn_reset_audio_settings = ButtonBuilder::new(WidgetBuilder::new()
                                .on_row(3)
                                .with_style(common_style.clone()))
                                .with_text("Reset")
                                .build(ui);
                            btn_reset_audio_settings
                        }))
                        .add_row(common_row)
                        .add_row(common_row)
                        .add_row(common_row)
                        .add_row(common_row)
                        .add_column(Column::strict(250.0))
                        .add_column(Column::stretch())
                        .build(ui)
                },
            })
            .with_tab(TabDefinition {
                header: {
                    TextBuilder::new(WidgetBuilder::new()
                        .with_width(100.0)
                        .with_height(30.0))
                        .with_text("Controls")
                        .build(ui)
                },
                content: {
                    let mut children = Vec::new();

                    for (row, button) in control_scheme.borrow().buttons().iter().enumerate() {
                        // Offset by total amount of rows that goes before
                        let row = row + 4;

                        let text = TextBuilder::new(WidgetBuilder::new()
                            .on_row(row)
                            .on_column(0)
                            .with_style(common_style.clone()))
                            .with_text(button.description.as_str())
                            .with_vertical_text_alignment(VerticalAlignment::Center)
                            .build(ui);
                        children.push(text);

                        let button = ButtonBuilder::new(WidgetBuilder::new()
                            .with_style(common_style.clone())
                            .on_row(row)
                            .on_column(1))
                            .with_text(button.button.name())
                            .build(ui);
                        children.push(button);
                        control_scheme_buttons.push(button);
                    }

                    GridBuilder::new(WidgetBuilder::new()
                        .with_child(TextBuilder::new(WidgetBuilder::new()
                            .on_row(0)
                            .on_column(0)
                            .with_style(common_style.clone()))
                            .with_text("Mouse Sensitivity")
                            .with_vertical_text_alignment(VerticalAlignment::Center)
                            .build(ui))
                        .with_child({
                            sb_mouse_sens = scroll_bar_template.instantiate(ui);
                            ui.node_mut(sb_mouse_sens)
                                .downcast_mut::<ScrollBar>()
                                .unwrap()
                                .set_min_value(0.05)
                                .set_max_value(2.0)
                                .set_value(control_scheme.borrow().mouse_sens)
                                .set_step(0.05)
                                .widget_mut()
                                .set_row(0)
                                .set_column(1);
                            sb_mouse_sens
                        })
                        .with_child(TextBuilder::new(WidgetBuilder::new()
                            .on_row(1)
                            .on_column(0)
                            .with_style(common_style.clone()))
                            .with_text("Inverse Mouse Y")
                            .with_vertical_text_alignment(VerticalAlignment::Center)
                            .build(ui))
                        .with_child({
                            cb_mouse_y_inverse = check_box_template.instantiate(ui);
                            ui.node_mut(cb_mouse_y_inverse)
                                .downcast_mut::<CheckBox>()
                                .unwrap()
                                .set_checked(Some(control_scheme.borrow().mouse_y_inverse))
                                .widget_mut()
                                .set_row(1)
                                .set_column(1);
                            cb_mouse_y_inverse
                        })
                        .with_child(TextBuilder::new(WidgetBuilder::new()
                            .on_row(2)
                            .on_column(0)
                            .with_style(common_style.clone()))
                            .with_text("Smooth Mouse")
                            .with_vertical_text_alignment(VerticalAlignment::Center)
                            .build(ui))
                        .with_child({
                            cb_smooth_mouse = check_box_template.instantiate(ui);
                            ui.node_mut(cb_smooth_mouse)
                                .downcast_mut::<CheckBox>()
                                .unwrap()
                                .set_checked(Some(control_scheme.borrow().smooth_mouse))
                                .widget_mut()
                                .set_row(2)
                                .set_column(1);
                            cb_smooth_mouse
                        })
                        .with_child(TextBuilder::new(WidgetBuilder::new()
                            .on_row(3)
                            .on_column(0)
                            .with_style(common_style.clone()))
                            .with_text("Shake Camera")
                            .with_vertical_text_alignment(VerticalAlignment::Center)
                            .build(ui))
                        .with_child({
                            cb_shake_camera = check_box_template.instantiate(ui);
                            ui.node_mut(cb_shake_camera)
                                .downcast_mut::<CheckBox>()
                                .unwrap()
                                .set_checked(Some(control_scheme.borrow().shake_camera))
                                .widget_mut()
                                .set_row(3)
                                .set_column(1);
                            cb_shake_camera
                        })
                        .with_child({
                            btn_reset_control_scheme = ButtonBuilder::new(WidgetBuilder::new()
                                .on_row(4 + control_scheme.borrow().buttons().len())
                                .with_style(common_style.clone()))
                                .with_text("Reset")
                                .build(ui);
                            btn_reset_control_scheme
                        })
                        .with_children(&children))
                        .add_column(Column::strict(250.0))
                        .add_column(Column::stretch())
                        .add_row(common_row)
                        .add_row(common_row)
                        .add_row(common_row)
                        .add_row(common_row)
                        .add_rows((0..control_scheme.borrow().buttons().len()).map(|_| common_row).collect())
                        .add_row(common_row)
                        .build(ui)
                },
            })
            .build(ui);

        let options_window: Handle<UINode> = WindowBuilder::new(WidgetBuilder::new()
            .with_width(500.0))
            .with_title(WindowTitle::Text("Options"))
            .open(false)
            .with_content(tab_control)
            .build(ui);

        let btn_new_game;
        let btn_settings;
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
            btn_settings,
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
            control_scheme,
            control_scheme_buttons,
            active_control_button: None,
            sb_mouse_sens,
            cb_mouse_y_inverse,
            cb_smooth_mouse,
            cb_shake_camera,
            btn_reset_control_scheme,
            cb_use_hrtf,
            btn_reset_audio_settings,
        }
    }

    pub fn set_visible(&mut self, ui: &mut UserInterface, visible: bool) {
        let visibility = if visible { Visibility::Visible } else { Visibility::Collapsed };
        ui.node_mut(self.root)
            .widget_mut()
            .set_visibility(visibility);

        if !visible {
            ui.send_event(UIEvent::targeted(self.options_window, UIEventKind::Closed));
        }
    }

    pub fn is_visible(&self, ui: &UserInterface) -> bool {
        ui.node(self.root)
            .widget()
            .get_visibility() == Visibility::Visible
    }

    pub fn process_input_event(&mut self, engine: &mut Engine, event: &Event<()>) {
        if let Event::WindowEvent { event, .. } = event {
            let mut control_button = None;

            match event {
                WindowEvent::Resized(new_size) => {
                    engine.renderer
                        .set_frame_size((*new_size).into())
                        .unwrap();
                    engine.user_interface
                        .node_mut(self.root)
                        .widget_mut()
                        .set_width(new_size.width as f32)
                        .set_height(new_size.height as f32);
                }
                WindowEvent::MouseWheel { delta, .. } => {
                    if let MouseScrollDelta::LineDelta(_, y) = delta {
                        if *y != 0.0 {
                            control_button = if *y >= 0.0 {
                                Some(ControlButton::WheelUp)
                            } else {
                                Some(ControlButton::WheelDown)
                            };
                        }
                    }
                }
                WindowEvent::KeyboardInput { input, .. } => {
                    if let Some(code) = input.virtual_keycode {
                        control_button = Some(ControlButton::Key(code));
                    }
                }
                WindowEvent::MouseInput { button, state, .. } => {
                    let index = match button {
                        MouseButton::Left => 1,
                        MouseButton::Right => 2,
                        MouseButton::Middle => 3,
                        MouseButton::Other(i) => *i,
                    };

                    control_button = Some(ControlButton::Mouse(index));
                }
                _ => {}
            }

            if let Some(control_button) = control_button {
                if let Some(active_control_button) = self.active_control_button {
                    let button_txt = engine.user_interface
                        .node_mut(self.control_scheme_buttons[active_control_button])
                        .downcast_mut::<Button>()
                        .unwrap()
                        .content();

                    engine.user_interface
                        .node_mut(button_txt)
                        .downcast_mut::<Text>()
                        .unwrap()
                        .set_text(control_button.name());

                    self.control_scheme
                        .borrow_mut()
                        .buttons_mut()[active_control_button]
                        .button = control_button;

                    self.active_control_button = None;
                }
            }
        }
    }

    fn sync_to_model(&mut self, engine: &mut Engine) {
        let ui = &mut engine.user_interface;
        let control_scheme = self.control_scheme.borrow();
        let settings = engine.renderer.get_quality_settings();

        let mut sync_check_box = |handle: Handle<UINode>, value: bool| {
            ui.node_mut(handle)
                .downcast_mut::<CheckBox>()
                .unwrap()
                .set_checked(Some(value));
        };
        sync_check_box(self.cb_spot_shadows, settings.spot_shadows_enabled);
        sync_check_box(self.cb_soft_spot_shadows, settings.spot_soft_shadows);
        sync_check_box(self.cb_point_shadows, settings.point_shadows_enabled);
        sync_check_box(self.cb_soft_point_shadows, settings.point_soft_shadows);
        sync_check_box(self.cb_mouse_y_inverse, control_scheme.mouse_y_inverse);
        sync_check_box(self.cb_smooth_mouse, control_scheme.smooth_mouse);
        sync_check_box(self.cb_shake_camera, control_scheme.shake_camera);
        let is_hrtf = if let rg3d::sound::renderer::Renderer::HrtfRenderer(_) = engine.sound_context.lock().unwrap().renderer() {
            true
        } else {
            false
        };
        sync_check_box(self.cb_use_hrtf, is_hrtf);

        let mut sync_scroll_bar = |handle: Handle<UINode>, value: f32| {
            ui.node_mut(handle)
                .downcast_mut::<ScrollBar>()
                .unwrap()
                .set_value(value);
        };
        sync_scroll_bar(self.sb_point_shadow_distance, settings.point_shadows_distance);
        sync_scroll_bar(self.sb_spot_shadow_distance, settings.spot_shadows_distance);
        sync_scroll_bar(self.sb_mouse_sens, control_scheme.mouse_sens);
        sync_scroll_bar(self.sb_sound_volume, engine.sound_context.lock().unwrap().master_gain());

        for (btn, def) in self.control_scheme_buttons.iter().zip(self.control_scheme.borrow().buttons().iter()) {
            let text = ui.node(*btn)
                .downcast_ref::<Button>()
                .unwrap()
                .content();

            ui.node_mut(text)
                .downcast_mut::<Text>()
                .unwrap()
                .set_text(def.button.name());
        }
    }

    pub fn handle_ui_event(&mut self, engine: &mut Engine, event: &UIEvent) {
        let old_settings = engine.renderer.get_quality_settings();
        let mut settings = old_settings;

        match event.kind {
            UIEventKind::NumericValueChanged { new_value, .. } => {
                if event.source() == self.sb_sound_volume {
                    engine.sound_context.lock().unwrap().set_master_gain(new_value)
                } else if event.source() == self.sb_point_shadow_distance {
                    settings.point_shadows_distance = new_value;
                } else if event.source() == self.sb_spot_shadow_distance {
                    settings.spot_shadows_distance = new_value;
                } else if event.source() == self.sb_mouse_sens {
                    self.control_scheme.borrow_mut().mouse_sens = new_value;
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
                let mut control_scheme = self.control_scheme.borrow_mut();
                if event.source() == self.cb_point_shadows {
                    settings.point_shadows_enabled = value.unwrap_or(false);
                } else if event.source() == self.cb_spot_shadows {
                    settings.spot_shadows_enabled = value.unwrap_or(false);
                } else if event.source() == self.cb_soft_spot_shadows {
                    settings.spot_soft_shadows = value.unwrap_or(false);
                } else if event.source() == self.cb_soft_point_shadows {
                    settings.point_soft_shadows = value.unwrap_or(false);
                } else if event.source() == self.cb_mouse_y_inverse {
                    control_scheme.mouse_y_inverse = value.unwrap_or(false);
                } else if event.source() == self.cb_smooth_mouse {
                    control_scheme.smooth_mouse = value.unwrap_or(false);
                } else if event.source() == self.cb_shake_camera {
                    control_scheme.shake_camera = value.unwrap_or(false);
                }
            }
            UIEventKind::Click => {
                if event.source() == self.btn_settings {
                    engine.user_interface
                        .send_event(UIEvent::targeted(self.options_window, UIEventKind::Opened));
                } else if event.source() == self.btn_reset_control_scheme {
                    self.control_scheme.borrow_mut().reset();
                    self.sync_to_model(engine);
                } else if event.source() == self.btn_reset_audio_settings {
                    engine.sound_context.lock().unwrap().set_master_gain(1.0);
                    self.sync_to_model(engine);
                }

                for (i, button) in self.control_scheme_buttons.iter().enumerate() {
                    if event.source() == *button {
                        let text = engine.user_interface
                            .node(*button)
                            .downcast_ref::<Button>()
                            .unwrap()
                            .content();

                        engine.user_interface
                            .node_mut(text)
                            .downcast_mut::<Text>()
                            .unwrap()
                            .set_text("[WAITING INPUT]");

                        self.active_control_button = Some(i);
                    }
                }
            }
            _ => ()
        }

        if settings != old_settings {
            if let Err(err) = engine.renderer.set_quality_settings(&settings) {
                println!("Failed to set renderer quality settings! Reason: {:?}", err);
            }
        }
    }
}