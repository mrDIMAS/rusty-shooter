use std::{
    rc::Rc,
    sync::mpsc::Sender,
    cell::RefCell,
};
use crate::{
    control_scheme::{
        ControlScheme,
        ControlButton,
    },
    message::Message,
    UINodeHandle,
    GameEngine,
    GuiMessage,
    gui::{
        create_check_box,
        create_scroll_bar,
        create_scroll_viewer
    },
};
use rg3d::{
    event::{
        WindowEvent,
        Event,
        MouseScrollDelta,
        MouseButton,
    },
    monitor::VideoMode,
    window::Fullscreen,
    gui::{
        items_control::ItemsControlBuilder,
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
        text::TextBuilder,
        border::BorderBuilder,
        decorator::DecoratorBuilder,
        button::ButtonBuilder,
        message::{
            UiMessageData,
            ScrollBarMessage,
            ItemsControlMessage,
            CheckBoxMessage,
            ButtonMessage,
        },
        scroll_bar::Orientation,
        widget::WidgetBuilder,
        HorizontalAlignment,
        Control,
        tab_control::{
            TabControlBuilder,
            TabDefinition,
        },
        node::UINode,
    },
};

pub struct OptionsMenu {
    pub window: UINodeHandle,
    sender: Sender<Message>,
    sb_sound_volume: UINodeHandle,
    pub sb_music_volume: UINodeHandle,
    lb_video_modes: UINodeHandle,
    cb_fullscreen: UINodeHandle,
    cb_spot_shadows: UINodeHandle,
    cb_soft_spot_shadows: UINodeHandle,
    cb_point_shadows: UINodeHandle,
    cb_soft_point_shadows: UINodeHandle,
    sb_point_shadow_distance: UINodeHandle,
    sb_spot_shadow_distance: UINodeHandle,
    video_modes: Vec<VideoMode>,
    control_scheme: Rc<RefCell<ControlScheme>>,
    control_scheme_buttons: Vec<UINodeHandle>,
    active_control_button: Option<usize>,
    sb_mouse_sens: UINodeHandle,
    cb_mouse_y_inverse: UINodeHandle,
    cb_smooth_mouse: UINodeHandle,
    cb_shake_camera: UINodeHandle,
    btn_reset_control_scheme: UINodeHandle,
    cb_use_hrtf: UINodeHandle,
    btn_reset_audio_settings: UINodeHandle,
}

impl OptionsMenu {
    pub fn new(engine: &mut GameEngine, control_scheme: Rc<RefCell<ControlScheme>>, sender: Sender<Message>) -> Self {
        let video_modes: Vec<VideoMode> = engine.get_window()
            .primary_monitor()
            .video_modes()
            .filter(|vm| vm.size().width > 800 &&
                vm.size().height > 600 && vm.bit_depth() == 32)
            .collect();

        let ui = &mut engine.user_interface;

        let common_row = Row::strict(36.0);

        let settings = engine.renderer.get_quality_settings();

        let margin = Thickness::uniform(2.0);

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
                            .with_margin(margin))
                            .with_text("Resolution")
                            .with_vertical_text_alignment(VerticalAlignment::Center)
                            .build(ui))
                        .with_child({
                            lb_video_modes = ItemsControlBuilder::new(WidgetBuilder::new()
                                .on_column(1)
                                .on_row(0)
                                .with_margin(margin))
                                .with_scroll_viewer(create_scroll_viewer(ui, &mut engine.resource_manager))
                                .with_items({
                                    let mut items = Vec::new();
                                    for video_mode in video_modes.iter() {
                                        let size = video_mode.size();
                                        let rate = video_mode.refresh_rate();
                                        let item = DecoratorBuilder::new(
                                            BorderBuilder::new(
                                                WidgetBuilder::new().with_child(
                                                    TextBuilder::new(WidgetBuilder::new()
                                                        .on_column(0)
                                                        .with_height(25.0)
                                                        .with_width(200.0))
                                                        .with_text(format!("{} x {} @ {}Hz", size.width, size.height, rate).as_str())
                                                        .with_vertical_text_alignment(VerticalAlignment::Center)
                                                        .with_horizontal_text_alignment(HorizontalAlignment::Center)
                                                        .build(ui))
                                            ).with_stroke_thickness(Thickness {
                                                left: 1.0,
                                                top: 0.0,
                                                right: 1.0,
                                                bottom: 1.0,
                                            }))
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
                            .with_margin(margin))
                            .with_text("Fullscreen")
                            .with_vertical_text_alignment(VerticalAlignment::Center)
                            .build(ui))
                        .with_child({
                            cb_fullscreen = create_check_box(ui, &mut engine.resource_manager);
                            if let UINode::CheckBox(check_box) = ui.node_mut(cb_fullscreen) {
                                check_box.set_checked(Some(false))
                                    .widget_mut()
                                    .set_row(1)
                                    .set_column(1);
                            }
                            cb_fullscreen
                        })

                        // Spot Shadows Enabled

                        .with_child(TextBuilder::new(WidgetBuilder::new()
                            .on_row(2)
                            .on_column(0)
                            .with_margin(margin))
                            .with_text("Spot Shadows")
                            .with_vertical_text_alignment(VerticalAlignment::Center)
                            .build(ui))
                        .with_child({
                            cb_spot_shadows = create_check_box(ui, &mut engine.resource_manager);
                            if let UINode::CheckBox(check_box) = ui.node_mut(cb_spot_shadows) {
                                check_box.set_checked(Some(settings.spot_shadows_enabled))
                                    .widget_mut()
                                    .set_row(2)
                                    .set_column(1);
                            }
                            cb_spot_shadows
                        })

                        // Soft Spot Shadows

                        .with_child(TextBuilder::new(WidgetBuilder::new()
                            .on_row(3)
                            .on_column(0)
                            .with_margin(margin))
                            .with_text("Soft Spot Shadows")
                            .with_vertical_text_alignment(VerticalAlignment::Center)
                            .build(ui))
                        .with_child({
                            cb_soft_spot_shadows = create_check_box(ui, &mut engine.resource_manager);
                            if let UINode::CheckBox(check_box) = ui.node_mut(cb_soft_spot_shadows) {
                                check_box.set_checked(Some(settings.spot_soft_shadows))
                                    .widget_mut()
                                    .set_row(3)
                                    .set_column(1);
                            }
                            cb_soft_spot_shadows
                        })

                        // Spot Shadows Distance

                        .with_child(TextBuilder::new(WidgetBuilder::new()
                            .on_row(4)
                            .on_column(0)
                            .with_margin(margin))
                            .with_text("Spot Shadows Distance")
                            .with_vertical_text_alignment(VerticalAlignment::Center)
                            .build(ui))
                        .with_child({
                            sb_spot_shadow_distance = create_scroll_bar(ui, &mut engine.resource_manager, Orientation::Horizontal, true);
                            if let UINode::ScrollBar(scroll_bar) = ui.node_mut(sb_spot_shadow_distance) {
                                scroll_bar.set_min_value(1.0)
                                    .set_max_value(15.0)
                                    .set_value(settings.spot_shadows_distance)
                                    .set_step(0.25)
                                    .widget_mut()
                                    .set_row(4)
                                    .set_column(1)
                                    .set_margin(Thickness::uniform(2.0));
                            }
                            sb_spot_shadow_distance
                        })

                        // Point Shadows Enabled

                        .with_child(TextBuilder::new(WidgetBuilder::new()
                            .on_row(5)
                            .on_column(0)
                            .with_margin(margin))
                            .with_text("Point Shadows")
                            .with_vertical_text_alignment(VerticalAlignment::Center)
                            .build(ui))
                        .with_child({
                            cb_point_shadows = create_check_box(ui, &mut engine.resource_manager);
                            if let UINode::CheckBox(check_box) = ui.node_mut(cb_point_shadows) {
                                check_box.set_checked(Some(settings.point_shadows_enabled))
                                    .widget_mut()
                                    .set_row(5)
                                    .set_column(1);
                            }
                            cb_point_shadows
                        })

                        // Soft Point Shadows

                        .with_child(TextBuilder::new(WidgetBuilder::new()
                            .on_row(6)
                            .on_column(0)
                            .with_margin(margin))
                            .with_text("Soft Point Shadows")
                            .with_vertical_text_alignment(VerticalAlignment::Center)
                            .build(ui))
                        .with_child({
                            cb_soft_point_shadows = create_check_box(ui, &mut engine.resource_manager);
                            if let UINode::CheckBox(check_box) = ui.node_mut(cb_soft_point_shadows) {
                                check_box.set_checked(Some(settings.point_soft_shadows))
                                    .widget_mut()
                                    .set_row(6)
                                    .set_column(1);
                            }
                            cb_soft_point_shadows
                        })

                        // Point Shadows Distance

                        .with_child(TextBuilder::new(WidgetBuilder::new()
                            .on_row(7)
                            .on_column(0)
                            .with_margin(margin))
                            .with_text("Point Shadows Distance")
                            .with_vertical_text_alignment(VerticalAlignment::Center)
                            .build(ui))
                        .with_child({
                            sb_point_shadow_distance = create_scroll_bar(ui, &mut engine.resource_manager, Orientation::Horizontal, true);
                            if let UINode::ScrollBar(scroll_bar) = ui.node_mut(sb_point_shadow_distance) {
                                scroll_bar.set_min_value(1.0)
                                    .set_max_value(15.0)
                                    .set_value(settings.point_shadows_distance)
                                    .set_step(0.25)
                                    .widget_mut()
                                    .set_row(7)
                                    .set_column(1)
                                    .set_margin(Thickness::uniform(2.0));
                            }
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
                            .with_margin(margin))
                            .with_text("Sound Volume")
                            .with_vertical_text_alignment(VerticalAlignment::Center)
                            .build(ui))
                        .with_child({
                            sb_sound_volume = create_scroll_bar(ui, &mut engine.resource_manager, Orientation::Horizontal, true);
                            if let UINode::ScrollBar(scroll_bar) = ui.node_mut(sb_sound_volume) {
                                scroll_bar.set_min_value(0.0)
                                    .set_max_value(1.0)
                                    .set_value(1.0)
                                    .set_step(0.025)
                                    .widget_mut()
                                    .set_row(0)
                                    .set_column(1)
                                    .set_margin(Thickness::uniform(2.0));
                            }
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
                            sb_music_volume = create_scroll_bar(ui, &mut engine.resource_manager, Orientation::Horizontal, true);
                            if let UINode::ScrollBar(scroll_bar) = ui.node_mut(sb_music_volume) {
                                scroll_bar.set_min_value(0.0)
                                    .set_max_value(1.0)
                                    .set_value(0.0)
                                    .set_step(0.025)
                                    .widget_mut()
                                    .set_row(1)
                                    .set_column(1)
                                    .set_margin(Thickness::uniform(2.0));
                            }
                            sb_music_volume
                        })
                        .with_child(TextBuilder::new(WidgetBuilder::new()
                            .on_row(2)
                            .on_column(0)
                            .with_margin(margin))
                            .with_text("Use HRTF")
                            .with_vertical_text_alignment(VerticalAlignment::Center)
                            .build(ui))
                        .with_child({
                            cb_use_hrtf = create_check_box(ui, &mut engine.resource_manager);
                            if let UINode::CheckBox(check_box) = ui.node_mut(cb_use_hrtf) {
                                check_box.set_checked(Some(true))
                                    .widget_mut()
                                    .set_row(2)
                                    .set_column(1);
                            }
                            cb_use_hrtf
                        })
                        .with_child({
                            btn_reset_audio_settings = ButtonBuilder::new(WidgetBuilder::new()
                                .on_row(3)
                                .with_margin(margin))
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
                            .with_margin(margin))
                            .with_text(button.description.as_str())
                            .with_vertical_text_alignment(VerticalAlignment::Center)
                            .build(ui);
                        children.push(text);

                        let button = ButtonBuilder::new(WidgetBuilder::new()
                            .with_margin(margin)
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
                            .with_margin(margin))
                            .with_text("Mouse Sensitivity")
                            .with_vertical_text_alignment(VerticalAlignment::Center)
                            .build(ui))
                        .with_child({
                            sb_mouse_sens = create_scroll_bar(ui, &mut engine.resource_manager, Orientation::Horizontal, true);
                            if let UINode::ScrollBar(scroll_bar) = ui.node_mut(sb_mouse_sens) {
                                scroll_bar.set_min_value(0.05)
                                    .set_max_value(2.0)
                                    .set_value(control_scheme.borrow().mouse_sens)
                                    .set_step(0.05)
                                    .widget_mut()
                                    .set_row(0)
                                    .set_column(1)
                                    .set_margin(Thickness::uniform(2.0));
                            }
                            sb_mouse_sens
                        })
                        .with_child(TextBuilder::new(WidgetBuilder::new()
                            .on_row(1)
                            .on_column(0)
                            .with_margin(margin))
                            .with_text("Inverse Mouse Y")
                            .with_vertical_text_alignment(VerticalAlignment::Center)
                            .build(ui))
                        .with_child({
                            cb_mouse_y_inverse = create_check_box(ui, &mut engine.resource_manager);
                            if let UINode::CheckBox(check_box) = ui.node_mut(cb_mouse_y_inverse) {
                                check_box.set_checked(Some(control_scheme.borrow().mouse_y_inverse))
                                    .widget_mut()
                                    .set_row(1)
                                    .set_column(1);
                            }
                            cb_mouse_y_inverse
                        })
                        .with_child(TextBuilder::new(WidgetBuilder::new()
                            .on_row(2)
                            .on_column(0)
                            .with_margin(margin))
                            .with_text("Smooth Mouse")
                            .with_vertical_text_alignment(VerticalAlignment::Center)
                            .build(ui))
                        .with_child({
                            cb_smooth_mouse = create_check_box(ui, &mut engine.resource_manager);
                            if let UINode::CheckBox(check_box) = ui.node_mut(cb_smooth_mouse) {
                                check_box.set_checked(Some(control_scheme.borrow().smooth_mouse))
                                    .widget_mut()
                                    .set_row(2)
                                    .set_column(1);
                            }
                            cb_smooth_mouse
                        })
                        .with_child(TextBuilder::new(WidgetBuilder::new()
                            .on_row(3)
                            .on_column(0)
                            .with_margin(margin))
                            .with_text("Shake Camera")
                            .with_vertical_text_alignment(VerticalAlignment::Center)
                            .build(ui))
                        .with_child({
                            cb_shake_camera = create_check_box(ui, &mut engine.resource_manager);
                            if let UINode::CheckBox(check_box) = ui.node_mut(cb_shake_camera) {
                                check_box.set_checked(Some(control_scheme.borrow().shake_camera))
                                    .widget_mut()
                                    .set_row(3)
                                    .set_column(1);
                            }
                            cb_shake_camera
                        })
                        .with_child({
                            btn_reset_control_scheme = ButtonBuilder::new(WidgetBuilder::new()
                                .on_row(4 + control_scheme.borrow().buttons().len())
                                .with_margin(margin))
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

        let options_window: UINodeHandle = WindowBuilder::new(WidgetBuilder::new()
            .with_width(500.0))
            .with_title(WindowTitle::Text("Options"))
            .open(false)
            .with_content(tab_control)
            .build(ui);

        Self {
            sender,
            window: options_window,
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

    pub fn sync_to_model(&mut self, engine: &mut GameEngine) {
        let ui = &mut engine.user_interface;
        let control_scheme = self.control_scheme.borrow();
        let settings = engine.renderer.get_quality_settings();

        let mut sync_check_box = |handle: UINodeHandle, value: bool| {
            if let UINode::CheckBox(check_box) = ui.node_mut(handle) {
                check_box.set_checked(Some(value));
            }
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

        let mut sync_scroll_bar = |handle: UINodeHandle, value: f32| {
            if let UINode::ScrollBar(scroll_bar) = ui.node_mut(handle) {
                scroll_bar.set_value(value);
            }
        };
        sync_scroll_bar(self.sb_point_shadow_distance, settings.point_shadows_distance);
        sync_scroll_bar(self.sb_spot_shadow_distance, settings.spot_shadows_distance);
        sync_scroll_bar(self.sb_mouse_sens, control_scheme.mouse_sens);
        sync_scroll_bar(self.sb_sound_volume, engine.sound_context.lock().unwrap().master_gain());

        for (btn, def) in self.control_scheme_buttons.iter().zip(self.control_scheme.borrow().buttons().iter()) {
            if let UINode::Button(button) = ui.node(*btn) {
                let text = button.content();
                if let UINode::Text(text) = ui.node_mut(text) {
                    text.set_text(def.button.name());
                }
            }
        }
    }

    pub fn process_input_event(&mut self, engine: &mut GameEngine, event: &Event<()>) {
        if let Event::WindowEvent { event, .. } = event {
            let mut control_button = None;

            match event {
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
                WindowEvent::MouseInput { button, .. } => {
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
                    if let UINode::Button(button) = engine.user_interface.node_mut(self.control_scheme_buttons[active_control_button]) {
                        let button_txt = button.content();
                        if let UINode::Text(button_txt) = engine.user_interface.node_mut(button_txt) {
                            button_txt.set_text(control_button.name());
                        }
                    }

                    self.control_scheme
                        .borrow_mut()
                        .buttons_mut()[active_control_button]
                        .button = control_button;

                    self.active_control_button = None;
                }
            }
        }
    }

    pub fn handle_ui_event(&mut self, engine: &mut GameEngine, message: &GuiMessage) {
        let old_settings = engine.renderer.get_quality_settings();
        let mut settings = old_settings;

        match &message.data {
            UiMessageData::ScrollBar(prop) => {
                if let ScrollBarMessage::Value(new_value) = prop {
                    if message.source() == self.sb_sound_volume {
                        engine.sound_context.lock().unwrap().set_master_gain(*new_value)
                    } else if message.source() == self.sb_point_shadow_distance {
                        settings.point_shadows_distance = *new_value;
                    } else if message.source() == self.sb_spot_shadow_distance {
                        settings.spot_shadows_distance = *new_value;
                    } else if message.source() == self.sb_mouse_sens {
                        self.control_scheme
                            .borrow_mut()
                            .mouse_sens = *new_value;
                    } else if message.source() == self.sb_music_volume {
                        self.sender
                            .send(Message::SetMusicVolume {
                                volume: *new_value
                            })
                            .unwrap();
                    }
                }
            }
            UiMessageData::ItemsControl(msg) => {
                if let ItemsControlMessage::SelectionChanged(new_value) = msg {
                    if message.source() == self.lb_video_modes {
                        if let Some(index) = new_value {
                            let video_mode = self.video_modes[*index].clone();
                            engine.get_window().set_fullscreen(Some(Fullscreen::Exclusive(video_mode)))
                        }
                    }
                }
            }
            UiMessageData::CheckBox(msg) => {
                if let CheckBoxMessage::Checked(value) = msg {
                    let mut control_scheme = self.control_scheme.borrow_mut();
                    if message.source() == self.cb_point_shadows {
                        settings.point_shadows_enabled = value.unwrap_or(false);
                    } else if message.source() == self.cb_spot_shadows {
                        settings.spot_shadows_enabled = value.unwrap_or(false);
                    } else if message.source() == self.cb_soft_spot_shadows {
                        settings.spot_soft_shadows = value.unwrap_or(false);
                    } else if message.source() == self.cb_soft_point_shadows {
                        settings.point_soft_shadows = value.unwrap_or(false);
                    } else if message.source() == self.cb_mouse_y_inverse {
                        control_scheme.mouse_y_inverse = value.unwrap_or(false);
                    } else if message.source() == self.cb_smooth_mouse {
                        control_scheme.smooth_mouse = value.unwrap_or(false);
                    } else if message.source() == self.cb_shake_camera {
                        control_scheme.shake_camera = value.unwrap_or(false);
                    }
                }
            }
            UiMessageData::Button(msg) => {
                if let ButtonMessage::Click = msg {
                    if message.source() == self.btn_reset_control_scheme {
                        self.control_scheme.borrow_mut().reset();
                        self.sync_to_model(engine);
                    } else if message.source() == self.btn_reset_audio_settings {
                        engine.sound_context.lock().unwrap().set_master_gain(1.0);
                        self.sync_to_model(engine);
                    }

                    for (i, button) in self.control_scheme_buttons.iter().enumerate() {
                        if message.source() == *button {
                            if let UINode::Button(button) = engine.user_interface.node(*button) {
                                let text = button.content();
                                if let UINode::Text(text) = engine.user_interface.node_mut(text) {
                                    text.set_text("[WAITING INPUT]");
                                }
                            }

                            self.active_control_button = Some(i);
                        }
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