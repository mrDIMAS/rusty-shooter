use crate::{
    character::Character,
    control_scheme::{ControlButton, ControlScheme},
    level::UpdateContext,
    message::Message,
};
use rand::Rng;
use rg3d::{
    core::{
        math::{mat3::Mat3, quat::Quat, vec3::Vec3},
        pool::Handle,
        visitor::{Visit, VisitResult, Visitor},
    },
    event::{DeviceEvent, ElementState, Event, MouseScrollDelta},
    physics::{
        convex_shape::{Axis, CapsuleShape, ConvexShape},
        rigid_body::RigidBody,
    },
    scene::{base::BaseBuilder, camera::CameraBuilder, node::Node, Scene},
    sound::context::Context,
};
use std::ops::{Deref, DerefMut};
use std::{
    cell::RefCell,
    rc::Rc,
    sync::{mpsc::Sender, Arc, Mutex},
};

pub struct Controller {
    move_forward: bool,
    move_backward: bool,
    move_left: bool,
    move_right: bool,
    crouch: bool,
    jump: bool,
    run: bool,
    shoot: bool,
}

impl Default for Controller {
    fn default() -> Controller {
        Controller {
            move_backward: false,
            move_forward: false,
            move_left: false,
            move_right: false,
            crouch: false,
            jump: false,
            run: false,
            shoot: false,
        }
    }
}

pub struct Player {
    character: Character,
    camera: Handle<Node>,
    camera_pivot: Handle<Node>,
    controller: Controller,
    yaw: f32,
    dest_yaw: f32,
    pitch: f32,
    dest_pitch: f32,
    run_speed_multiplier: f32,
    crouch_speed_multiplier: f32,
    stand_body_height: f32,
    crouch_body_height: f32,
    move_speed: f32,
    camera_offset: Vec3,
    camera_dest_offset: Vec3,
    path_len: f32,
    feet_position: Vec3,
    head_position: Vec3,
    look_direction: Vec3,
    up_direction: Vec3,
    weapon_offset: Vec3,
    weapon_dest_offset: Vec3,
    weapon_shake_factor: f32,
    crouch_speed: f32,
    stand_up_speed: f32,
    listener_basis: Mat3,
    control_scheme: Option<Rc<RefCell<ControlScheme>>>,
}

impl Deref for Player {
    type Target = Character;

    fn deref(&self) -> &Self::Target {
        &self.character
    }
}

impl DerefMut for Player {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.character
    }
}

impl Default for Player {
    fn default() -> Self {
        Self {
            character: Default::default(),
            camera: Default::default(),
            camera_pivot: Default::default(),
            controller: Controller::default(),
            stand_body_height: 0.5,
            dest_pitch: 0.0,
            dest_yaw: 0.0,
            move_speed: 0.058,
            run_speed_multiplier: 1.75,
	    crouch_speed_multiplier: 0.5,
            crouch_body_height: 0.04,
            yaw: 0.0,
            pitch: 0.0,
            camera_dest_offset: Vec3::ZERO,
            camera_offset: Vec3::ZERO,
            path_len: 0.0,
            feet_position: Vec3::ZERO,
            head_position: Vec3::ZERO,
            look_direction: Vec3::ZERO,
            up_direction: Vec3::ZERO,
            weapon_offset: Default::default(),
            weapon_dest_offset: Default::default(),
            weapon_shake_factor: 0.0,
            crouch_speed: 0.15,
            stand_up_speed: 0.12,
            listener_basis: Default::default(),
            control_scheme: None,
        }
    }
}

impl Visit for Player {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.character.visit("Character", visitor)?;
        self.camera.visit("Camera", visitor)?;
        self.camera_pivot.visit("CameraPivot", visitor)?;
        self.yaw.visit("Yaw", visitor)?;
        self.dest_yaw.visit("DestYaw", visitor)?;
        self.pitch.visit("Pitch", visitor)?;
        self.dest_pitch.visit("DestPitch", visitor)?;
        self.run_speed_multiplier
            .visit("RunSpeedMultiplier", visitor)?;
        self.stand_body_height.visit("StandBodyRadius", visitor)?;
        self.crouch_body_height.visit("CrouchBodyRadius", visitor)?;
        self.move_speed.visit("MoveSpeed", visitor)?;
        self.camera_offset.visit("CameraOffset", visitor)?;
        self.camera_dest_offset.visit("CameraDestOffset", visitor)?;

        visitor.leave_region()
    }
}

impl Player {
    pub fn new(scene: &mut Scene, sender: Sender<Message>) -> Player {
        let camera_handle = scene
            .graph
            .add_node(Node::Camera(CameraBuilder::new(BaseBuilder::new()).build()));

        let height = Self::default().stand_body_height;
        let mut camera_pivot = Node::Base(Default::default());
        camera_pivot.local_transform_mut().set_position(Vec3 {
            x: 0.0,
            y: height - 0.20,
            z: 0.0,
        });
        let camera_pivot_handle = scene.graph.add_node(camera_pivot);
        scene.graph.link_nodes(camera_handle, camera_pivot_handle);

        let mut pivot = Node::Base(Default::default());
        pivot.local_transform_mut().set_position(Vec3 {
            x: -1.0,
            y: 0.0,
            z: 1.0,
        });

        let capsule_shape = CapsuleShape::new(0.35, height, Axis::Y);
        let mut body = RigidBody::new(ConvexShape::Capsule(capsule_shape));
        body.set_friction(Vec3::new(0.2, 0.0, 0.2));
        let body_handle = scene.physics.add_body(body);
        let pivot_handle = scene.graph.add_node(pivot);
        scene.physics_binder.bind(pivot_handle, body_handle);
        scene.graph.link_nodes(camera_pivot_handle, pivot_handle);

        let mut weapon_base_pivot = Node::Base(Default::default());
        weapon_base_pivot
            .local_transform_mut()
            .set_position(Vec3::new(-0.035, -0.052, 0.02));
        let weapon_base_pivot_handle = scene.graph.add_node(weapon_base_pivot);
        scene
            .graph
            .link_nodes(weapon_base_pivot_handle, camera_handle);

        let weapon_pivot = Node::Base(Default::default());
        let weapon_pivot_handle = scene.graph.add_node(weapon_pivot);
        scene
            .graph
            .link_nodes(weapon_pivot_handle, weapon_base_pivot_handle);

        Player {
            character: Character {
                pivot: pivot_handle,
                body: body_handle,
                weapon_pivot: weapon_pivot_handle,
                sender: Some(sender),
                name: "Player".to_owned(),
                ..Default::default()
            },
            camera: camera_handle,
            camera_pivot: camera_pivot_handle,
            ..Default::default()
        }
    }

    fn handle_crouch(&mut self, body: &mut RigidBody) {
        let capsule = body.get_shape_mut().as_capsule_mut();
        let current_height = capsule.get_height();
        let new_height = if self.controller.crouch {
            let new_height = current_height - self.crouch_speed;
            if new_height < self.crouch_body_height {
                self.crouch_body_height
            } else {
                new_height
            }
        } else {
            let new_height = current_height + self.stand_up_speed;
            if new_height > self.stand_body_height {
                self.stand_body_height
            } else {
                new_height
            }
        };
        capsule.set_height(new_height);
    }

    pub fn camera(&self) -> Handle<Node> {
        self.camera
    }

    pub fn set_control_scheme(&mut self, control_scheme: Rc<RefCell<ControlScheme>>) {
        self.control_scheme = Some(control_scheme);
    }

    /// Mathematical function that tries to simulate the natural up-and-down shaking of the line of sight when you move
    /// (a.k.a. "view bobbing")
    fn view_bobbing_function(a: f32, b: f32, c: f32, d: f32) -> f32 {
	a * (b * (-c).sin() - d * c).sin()
    }

    fn player_walk(&mut self, pivot: &Node, time_elapsed: f64, body: &mut RigidBody) {
        let look = pivot.look_vector();
        let side = pivot.side_vector();

        let mut velocity = Vec3::ZERO;
        if self.controller.move_forward {
	    velocity += look;
        }
	if self.controller.move_backward {
	    velocity -= look;
        }
	if self.controller.move_left {
	    velocity += side;
        }
	if self.controller.move_right {
	    velocity -= side;
        }

        if let Some(normalized_velocity) = velocity.normalized() {
	    let speed_mult = if self.controller.crouch {
		self.crouch_speed_multiplier
	    } else if self.controller.run {
		self.run_speed_multiplier
	    } else {
		1.0
	    };

	    body.set_x_velocity(normalized_velocity.x * self.move_speed * speed_mult);
	    body.set_z_velocity(normalized_velocity.z * self.move_speed * speed_mult);

	    let k = (time_elapsed * 15.0) as f32;

	    // Weapon bobbing (don't bob when crouching)
	    if !self.controller.crouch {
		self.weapon_dest_offset.y = Self::view_bobbing_function((0.0001 * self.move_speed).sqrt(), -1.5, 0.5 * k - 2.0, 0.8);
		self.weapon_shake_factor += 0.23;
	    }

	    // View bobbing
	    self.camera_dest_offset.y = Self::view_bobbing_function((0.3 * self.move_speed).sqrt(), -1.5, 0.5 * k, 1.0);
	    self.path_len += 0.1;
        } else {
	    self.weapon_dest_offset = Vec3::ZERO;
        }
    }

    fn update_movement(&mut self, context: &mut UpdateContext) {
        let has_ground_contact = self.character.has_ground_contact(&context.scene.physics);
        let body = context.scene.physics.borrow_body_mut(self.character.body);

	if has_ground_contact {
	    self.player_walk(&context.scene.graph[self.character.pivot], context.time.elapsed, body);
	}

        self.weapon_offset.follow(&self.weapon_dest_offset, 0.1);

        context.scene.graph[self.character.weapon_pivot]
            .local_transform_mut()
            .set_position(self.weapon_offset);

        if self.controller.jump {
            if has_ground_contact {
                body.set_y_velocity(0.07);
            }
            self.controller.jump = false;
        }

        self.handle_crouch(body);

        self.feet_position = body.get_position();
        self.feet_position.y -= body.get_shape().as_capsule().get_height();

        if self.control_scheme.as_ref().unwrap().borrow().shake_camera {
            self.camera_offset.follow(&self.camera_dest_offset, 0.1);
        } else {
            self.camera_offset = Vec3::ZERO;
        }

        let camera_node = &mut context.scene.graph[self.camera];
        camera_node
            .local_transform_mut()
            .set_position(self.camera_offset);

        self.head_position = camera_node.global_position();
        self.look_direction = camera_node.look_vector();
        self.up_direction = camera_node.up_vector();
        self.listener_basis = Mat3::from_vectors(
            camera_node.side_vector(),
            camera_node.up_vector(),
            -camera_node.look_vector(),
        );

        if self.control_scheme.clone().unwrap().borrow().smooth_mouse {
            self.yaw += (self.dest_yaw - self.yaw) * 0.2;
            self.pitch += (self.dest_pitch - self.pitch) * 0.2;
        } else {
            self.yaw = self.dest_yaw;
            self.pitch = self.dest_pitch;
        }

        context.scene.graph[self.character.pivot]
            .local_transform_mut()
            .set_rotation(Quat::from_axis_angle(Vec3::UP, self.yaw.to_radians()));

        context.scene.graph[self.camera_pivot]
            .local_transform_mut()
            .set_rotation(Quat::from_axis_angle(Vec3::RIGHT, self.pitch.to_radians()));
    }

    fn update_listener(&mut self, sound_context: Arc<Mutex<Context>>) {
        let mut sound_context = sound_context.lock().unwrap();
        let listener = sound_context.listener_mut();
        listener.set_basis(self.listener_basis);
        listener.set_position(self.head_position);
    }

    pub fn can_be_removed(&self) -> bool {
        self.character.is_dead()
    }

    #[allow(clippy::cognitive_complexity)]
    pub fn process_input_event(&mut self, event: &Event<()>) -> bool {
        if let Event::DeviceEvent { event, .. } = event {
            if let Some(control_scheme) = self.control_scheme.clone() {
                let control_scheme = control_scheme.borrow();

                let mut control_button = None;
                let mut control_button_state = ElementState::Released;

                match event {
                    DeviceEvent::MouseMotion { delta } => {
                        self.dest_yaw -= delta.0 as f32 * control_scheme.mouse_sens;

                        let sens = if control_scheme.mouse_y_inverse {
                            -control_scheme.mouse_sens
                        } else {
                            control_scheme.mouse_sens
                        };

                        self.dest_pitch += delta.1 as f32 * sens;
                        if self.dest_pitch > 90.0 {
                            self.dest_pitch = 90.0;
                        } else if self.dest_pitch < -90.0 {
                            self.dest_pitch = -90.0;
                        }
                    }

                    DeviceEvent::Button { button, state } => {
                        control_button = Some(ControlButton::Mouse(*button as u8));
                        control_button_state = *state;
                    }

                    DeviceEvent::Key(input) => {
                        if let Some(code) = input.virtual_keycode {
                            control_button = Some(ControlButton::Key(code));
                            control_button_state = input.state;
                        }
                    }

                    DeviceEvent::MouseWheel { delta } => {
                        if let MouseScrollDelta::LineDelta(_, y) = delta {
                            if *y < 0.0 {
                                self.prev_weapon();
                            } else if *y > 0.0 {
                                self.next_weapon();
                            }
                        }
                    }

                    _ => (),
                }

                if let Some(control_button) = control_button {
                    match control_button_state {
                        ElementState::Pressed => {
                            if control_button == control_scheme.shoot.button {
                                self.controller.shoot = true;
                            } else if control_button == control_scheme.move_forward.button {
                                self.controller.move_forward = true;
                            } else if control_button == control_scheme.move_backward.button {
                                self.controller.move_backward = true;
                            } else if control_button == control_scheme.move_left.button {
                                self.controller.move_left = true;
                            } else if control_button == control_scheme.move_right.button {
                                self.controller.move_right = true;
                            } else if control_button == control_scheme.crouch.button {
                                self.controller.crouch = true;
                            } else if control_button == control_scheme.run.button {
                                self.controller.run = true;
                            } else if control_button == control_scheme.jump.button {
                                self.controller.jump = true;
                            }
                        }
                        ElementState::Released => {
                            if control_button == control_scheme.shoot.button {
                                self.controller.shoot = false;
                            } else if control_button == control_scheme.move_forward.button {
                                self.controller.move_forward = false;
                            } else if control_button == control_scheme.move_backward.button {
                                self.controller.move_backward = false;
                            } else if control_button == control_scheme.move_left.button {
                                self.controller.move_left = false;
                            } else if control_button == control_scheme.move_right.button {
                                self.controller.move_right = false;
                            } else if control_button == control_scheme.crouch.button {
                                self.controller.crouch = false;
                            } else if control_button == control_scheme.run.button {
                                self.controller.run = false;
                            }
                        }
                    }
                }
            }
        }

        false
    }

    pub fn update(&mut self, context: &mut UpdateContext) {
        self.update_movement(context);

        if let Some(current_weapon_handle) = self
            .character
            .weapons
            .get(self.character.current_weapon as usize)
        {
            let velocity = context
                .scene
                .physics
                .borrow_body(self.character.body)
                .get_velocity();

            if self.controller.shoot {
                self.character
                    .sender
                    .as_ref()
                    .unwrap()
                    .send(Message::ShootWeapon {
                        weapon: *current_weapon_handle,
                        initial_velocity: velocity,
                        direction: None,
                    })
                    .unwrap();
            }
        }

        if self.path_len > 2.0 {
            let footsteps = [
                "data/sounds/footsteps/FootStep_shoe_stone_step1.wav",
                "data/sounds/footsteps/FootStep_shoe_stone_step2.wav",
                "data/sounds/footsteps/FootStep_shoe_stone_step3.wav",
                "data/sounds/footsteps/FootStep_shoe_stone_step4.wav",
            ];
            self.character
                .sender
                .as_ref()
                .unwrap()
                .send(Message::PlaySound {
                    path: footsteps[rand::thread_rng().gen_range(0, footsteps.len())].into(),
                    position: self.character.position(&context.scene.physics),
                    gain: 1.0,
                    rolloff_factor: 2.0,
                    radius: 3.0,
                })
                .unwrap();

            self.path_len = 0.0;
        }

        self.update_listener(context.sound_context.clone());
    }

    pub fn clean_up(&mut self, scene: &mut Scene) {
        self.character.clean_up(scene)
    }
}
