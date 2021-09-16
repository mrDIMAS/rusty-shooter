use crate::{
    character::Character,
    control_scheme::{ControlButton, ControlScheme},
    level::UpdateContext,
    message::Message,
};
use rg3d::sound::context::SoundContext;
use rg3d::{
    core::rand::Rng,
    core::{
        algebra::{Matrix3, UnitQuaternion, Vector3},
        math::Vector3Ext,
        pool::Handle,
        visitor::{Visit, VisitResult, Visitor},
    },
    event::{DeviceEvent, ElementState, Event, MouseScrollDelta, WindowEvent},
    physics3d::rapier::{
        dynamics::{BodyStatus, RigidBodyBuilder},
        geometry::ColliderBuilder,
    },
    rand,
    scene::transform::TransformBuilder,
    scene::{base::BaseBuilder, camera::CameraBuilder, node::Node, Scene},
};
use std::{
    ops::{Deref, DerefMut},
    sync::{mpsc::Sender, Arc, RwLock},
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
    stand_body_height: f32,
    crouch_body_height: f32,
    move_speed: f32,
    camera_offset: Vector3<f32>,
    camera_dest_offset: Vector3<f32>,
    path_len: f32,
    feet_position: Vector3<f32>,
    head_position: Vector3<f32>,
    look_direction: Vector3<f32>,
    up_direction: Vector3<f32>,
    weapon_offset: Vector3<f32>,
    weapon_dest_offset: Vector3<f32>,
    weapon_shake_factor: f32,
    crouch_speed: f32,
    stand_up_speed: f32,
    listener_basis: Matrix3<f32>,
    control_scheme: Option<Arc<RwLock<ControlScheme>>>,
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
            stand_body_height: 1.05,
            dest_pitch: 0.0,
            dest_yaw: 0.0,
            move_speed: 3.48,
            run_speed_multiplier: 1.75,
            crouch_body_height: 0.15,
            yaw: 0.0,
            pitch: 0.0,
            camera_dest_offset: Vector3::default(),
            camera_offset: Vector3::default(),
            path_len: 0.0,
            feet_position: Vector3::default(),
            head_position: Vector3::default(),
            look_direction: Vector3::default(),
            up_direction: Vector3::default(),
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
        let height = Self::default().stand_body_height;

        let body_handle = scene.physics.add_body(
            RigidBodyBuilder::new(BodyStatus::Dynamic)
                .can_sleep(false)
                .build(),
        );
        scene.physics.add_collider(
            ColliderBuilder::capsule_y(height * 0.5, 0.35)
                .friction(0.0)
                .build(),
            &body_handle,
        );

        let camera_handle;
        let camera_pivot_handle;
        let weapon_base_pivot_handle;
        let weapon_pivot_handle;
        let pivot_handle = BaseBuilder::new()
            .with_children(&[{
                camera_pivot_handle = BaseBuilder::new()
                    .with_children(&[{
                        camera_handle = CameraBuilder::new(BaseBuilder::new().with_children(&[{
                            weapon_base_pivot_handle = BaseBuilder::new()
                                .with_children(&[{
                                    weapon_pivot_handle =
                                        BaseBuilder::new().build(&mut scene.graph);
                                    weapon_pivot_handle
                                }])
                                .with_local_transform(
                                    TransformBuilder::new()
                                        .with_local_position(Vector3::new(-0.065, -0.052, 0.02))
                                        .build(),
                                )
                                .build(&mut scene.graph);
                            weapon_base_pivot_handle
                        }]))
                        .build(&mut scene.graph);
                        camera_handle
                    }])
                    .with_local_transform(
                        TransformBuilder::new()
                            .with_local_position(Vector3::new(0.0, height - 0.20, 0.0))
                            .build(),
                    )
                    .build(&mut scene.graph);
                camera_pivot_handle
            }])
            .build(&mut scene.graph);

        scene.physics_binder.bind(pivot_handle, body_handle.into());

        Player {
            character: Character {
                pivot: pivot_handle,
                body: body_handle.into(),
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

    // TODO: rapier does not support scaling of collider yet.
    /*
    fn handle_crouch(&mut self, body: &mut RigidBody, physics: &mut Physics) {
        let capsule = body.get_shape_mut().as_capsule_mut();
        let current_height = capsule.get_height();
        if self.controller.crouch {
            let new_height = current_height - self.crouch_speed;
            if new_height < self.crouch_body_height {
                capsule.set_height(self.crouch_body_height);
            } else {
                capsule.set_height(new_height);
            }
        } else {
            let new_height = (current_height + self.stand_up_speed).min(self.stand_body_height);
            // Divide by 2.0 because we want to know offset of cap of capsule relative to its center.
            let offset = (new_height - capsule.get_height()) / 2.0;
            capsule.set_height(new_height);

            // Prevent "jumping" when standing up. This happens because when player stands on ground
            // lower cap of its body's capsule touches the ground, but when we increase height, its
            // cap become under the ground and physics engine will push it out adding some momentum
            // to it which will look like a jump.

            // Cache velocity because it is calculated using position from previous frame.
            let vel = body.get_velocity();
            // Push body up.
            body.set_position(body.get_position() + Vector3::new(0.0, offset, 0.0));
            // Set new velocity. We divide offset by FIXED_FPS because we need to find speed
            // and its units are (units/frame - units per frame).
            body.set_velocity(vel - Vector3::new(0.0, offset / FIXED_FPS, 0.0));
        };
    }*/

    pub fn camera(&self) -> Handle<Node> {
        self.camera
    }

    pub fn set_control_scheme(&mut self, control_scheme: Arc<RwLock<ControlScheme>>) {
        self.control_scheme = Some(control_scheme);
    }

    fn update_movement(&mut self, context: &mut UpdateContext) {
        let pivot = &context.scene.graph[self.character.pivot];
        let look = pivot.look_vector();
        let side = pivot.side_vector();

        let has_ground_contact = self.character.has_ground_contact(&context.scene.physics);

        let mut velocity = Vector3::default();
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

        let speed_mult = if self.controller.run {
            self.run_speed_multiplier
        } else {
            1.0
        };

        let body = context
            .scene
            .physics
            .bodies
            .get_mut(&self.character.body)
            .unwrap();
        body.set_angvel(Default::default(), true);
        if let Some(normalized_velocity) = velocity.try_normalize(std::f32::EPSILON) {
            body.set_linvel(
                Vector3::new(
                    normalized_velocity.x * self.move_speed * speed_mult,
                    body.linvel().y,
                    normalized_velocity.z * self.move_speed * speed_mult,
                ),
                true,
            );

            self.weapon_dest_offset.x = 0.01 * (self.weapon_shake_factor * 0.5).cos();
            self.weapon_dest_offset.y = 0.005 * self.weapon_shake_factor.sin();
            self.weapon_shake_factor += 0.23;

            if has_ground_contact {
                let k = (context.time.elapsed * 15.0) as f32;
                self.camera_dest_offset.x = 0.05 * (k * 0.5).cos();
                self.camera_dest_offset.y = 0.1 * k.sin();
                self.path_len += 0.1;
            }
        } else {
            self.weapon_dest_offset = Vector3::default();
        }

        // Damping to prevent sliding.
        // TODO: This is needed because Rapier does not have selection of friction
        // models yet.
        if has_ground_contact {
            let mut vel = *body.linvel();
            vel.x *= 0.9;
            vel.z *= 0.9;
            body.set_linvel(vel, true);
        }

        self.weapon_offset.follow(&self.weapon_dest_offset, 0.1);

        context.scene.graph[self.character.weapon_pivot]
            .local_transform_mut()
            .set_position(self.weapon_offset);

        if self.controller.jump {
            if has_ground_contact {
                let mut vel = *body.linvel();
                vel.y = 4.2;
                body.set_linvel(vel, true);
            }
            self.controller.jump = false;
        }

        //self.handle_crouch(body);

        self.feet_position = body.position().translation.vector;
        self.feet_position.y -= self.stand_body_height;

        if self
            .control_scheme
            .as_ref()
            .unwrap()
            .read()
            .unwrap()
            .shake_camera
        {
            self.camera_offset.follow(&self.camera_dest_offset, 0.1);
        } else {
            self.camera_offset = Vector3::default();
        }

        let camera_node = &mut context.scene.graph[self.camera];
        camera_node
            .local_transform_mut()
            .set_position(self.camera_offset);

        self.head_position = camera_node.global_position();
        self.look_direction = camera_node.look_vector();
        self.up_direction = camera_node.up_vector();
        self.listener_basis = Matrix3::from_columns(&[
            camera_node.side_vector(),
            camera_node.up_vector(),
            -camera_node.look_vector(),
        ]);

        if self
            .control_scheme
            .clone()
            .unwrap()
            .read()
            .unwrap()
            .smooth_mouse
        {
            self.yaw += (self.dest_yaw - self.yaw) * 0.2;
            self.pitch += (self.dest_pitch - self.pitch) * 0.2;
        } else {
            self.yaw = self.dest_yaw;
            self.pitch = self.dest_pitch;
        }

        let mut position = *body.position();
        position.rotation =
            UnitQuaternion::from_axis_angle(&Vector3::y_axis(), self.yaw.to_radians());
        body.set_position(position, true);

        context.scene.graph[self.camera_pivot]
            .local_transform_mut()
            .set_rotation(UnitQuaternion::from_axis_angle(
                &Vector3::x_axis(),
                self.pitch.to_radians(),
            ));
    }

    fn update_listener(&mut self, sound_context: SoundContext) {
        let mut sound_context = sound_context.state();
        let listener = sound_context.listener_mut();
        listener.set_basis(self.listener_basis);
        listener.set_position(self.head_position);
    }

    pub fn can_be_removed(&self) -> bool {
        self.character.is_dead()
    }

    #[allow(clippy::cognitive_complexity)]
    pub fn process_input_event(&mut self, event: &Event<()>) -> bool {
        let control_scheme = match self.control_scheme.clone() {
            Some(x) => x,
            None => return false,
        };
        let control_scheme = control_scheme.read().unwrap();

        let mut control_button = None;
        let mut control_button_state = ElementState::Released;

        // get mouse input
        if let Event::DeviceEvent { event, .. } = event {
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
                    control_button = Some(ControlButton::Mouse(*button as u16));
                    control_button_state = *state;
                }

                DeviceEvent::Key(_input) => {
                    // handle keyboard input via `WindowEvent` considering winit issue on macOS
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
        }

        // get keyboard input
        if let Event::WindowEvent { event, .. } = event {
            if let WindowEvent::KeyboardInput { input, .. } = event {
                if let Some(code) = input.virtual_keycode {
                    control_button = Some(ControlButton::Key(code));
                    control_button_state = input.state;
                }
            }
        }

        // apply input
        let control_button = match control_button {
            Some(x) => x,
            None => return false,
        };

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
                .bodies
                .get(&self.character.body)
                .unwrap()
                .linvel();

            if self.controller.shoot {
                self.character
                    .sender
                    .as_ref()
                    .unwrap()
                    .send(Message::ShootWeapon {
                        weapon: *current_weapon_handle,
                        initial_velocity: *velocity,
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
                    path: footsteps[rand::thread_rng().gen_range(0..footsteps.len())].into(),
                    position: self.character.position(&context.scene.physics),
                    gain: 1.0,
                    rolloff_factor: 2.0,
                    radius: 3.0,
                })
                .unwrap();

            self.path_len = 0.0;
        }

        self.update_listener(context.scene.sound_context.clone());
    }

    pub fn clean_up(&mut self, scene: &mut Scene) {
        self.character.clean_up(scene)
    }
}
