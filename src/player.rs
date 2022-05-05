use crate::{
    character::Character,
    control_scheme::{ControlButton, ControlScheme},
    level::UpdateContext,
    message::Message,
};
use fyrox::{
    core::{
        algebra::{Matrix3, UnitQuaternion, Vector3},
        math::Vector3Ext,
        pool::Handle,
        rand::Rng,
        visitor::{Visit, VisitResult, Visitor},
    },
    event::{DeviceEvent, ElementState, Event, MouseScrollDelta, WindowEvent},
    rand,
    scene::{
        base::BaseBuilder,
        camera::CameraBuilder,
        collider::{ColliderBuilder, ColliderShape},
        graph::physics::CoefficientCombineRule,
        node::Node,
        pivot::PivotBuilder,
        rigidbody::{RigidBodyBuilder, RigidBodyType},
        sound::listener::ListenerBuilder,
        transform::TransformBuilder,
        Scene,
    },
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

#[derive(Visit)]
pub struct Player {
    character: Character,
    camera: Handle<Node>,
    camera_pivot: Handle<Node>,
    #[visit(skip)]
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
    #[visit(skip)]
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
            control_scheme: None,
        }
    }
}

impl Player {
    pub fn new(scene: &mut Scene, sender: Sender<Message>) -> Player {
        let height = Self::default().stand_body_height;

        let camera_handle;
        let camera_pivot_handle;
        let weapon_base_pivot_handle;
        let weapon_pivot_handle;
        let collider;
        let body_handle = RigidBodyBuilder::new(BaseBuilder::new().with_children(&[
            {
                collider = ColliderBuilder::new(BaseBuilder::new())
                    .with_shape(ColliderShape::capsule_y(height * 0.5, 0.35))
                    .with_friction_combine_rule(CoefficientCombineRule::Min)
                    .build(&mut scene.graph);
                collider
            },
            {
                camera_pivot_handle = PivotBuilder::new(
                    BaseBuilder::new()
                        .with_children(&[{
                            camera_handle = CameraBuilder::new(
                                BaseBuilder::new().with_children(&[
                                    {
                                        weapon_base_pivot_handle = PivotBuilder::new(
                                            BaseBuilder::new()
                                                .with_children(&[{
                                                    weapon_pivot_handle =
                                                        PivotBuilder::new(BaseBuilder::new())
                                                            .build(&mut scene.graph);
                                                    weapon_pivot_handle
                                                }])
                                                .with_local_transform(
                                                    TransformBuilder::new()
                                                        .with_local_position(Vector3::new(
                                                            -0.065, -0.052, 0.02,
                                                        ))
                                                        .build(),
                                                ),
                                        )
                                        .build(&mut scene.graph);
                                        weapon_base_pivot_handle
                                    },
                                    ListenerBuilder::new(BaseBuilder::new())
                                        .build(&mut scene.graph),
                                ]),
                            )
                            .build(&mut scene.graph);
                            camera_handle
                        }])
                        .with_local_transform(
                            TransformBuilder::new()
                                .with_local_position(Vector3::new(0.0, height - 0.20, 0.0))
                                .build(),
                        ),
                )
                .build(&mut scene.graph);
                camera_pivot_handle
            },
        ]))
        .with_locked_rotations(true)
        .with_can_sleep(false)
        .with_body_type(RigidBodyType::Dynamic)
        .build(&mut scene.graph);

        Player {
            character: Character {
                body: body_handle,
                collider,
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
        let has_ground_contact = self.character.has_ground_contact(&context.scene.graph);

        let body = context.scene.graph[self.character.body].as_rigid_body_mut();
        let look = body.look_vector();
        let side = body.side_vector();

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

        if let Some(normalized_velocity) = velocity.try_normalize(std::f32::EPSILON) {
            body.set_lin_vel(Vector3::new(
                normalized_velocity.x * self.move_speed * speed_mult,
                body.lin_vel().y,
                normalized_velocity.z * self.move_speed * speed_mult,
            ));

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

        self.weapon_offset.follow(&self.weapon_dest_offset, 0.1);

        if self.controller.jump {
            if has_ground_contact {
                let mut vel = body.lin_vel();
                vel.y = 4.2;
                body.set_lin_vel(vel);
            }
            self.controller.jump = false;
        }

        // Apply damping in XZ plane to prevent sliding.
        if has_ground_contact {
            let mut lin_vel = body.lin_vel();
            lin_vel.x *= 0.9;
            lin_vel.z *= 0.9;
            body.set_lin_vel(lin_vel);
        }

        //self.handle_crouch(body);

        self.feet_position = body.global_position();
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

        body.local_transform_mut()
            .set_rotation(UnitQuaternion::from_axis_angle(
                &Vector3::y_axis(),
                self.yaw.to_radians(),
            ));

        context.scene.graph[self.camera_pivot]
            .local_transform_mut()
            .set_rotation(UnitQuaternion::from_axis_angle(
                &Vector3::x_axis(),
                self.pitch.to_radians(),
            ));

        context.scene.graph[self.character.weapon_pivot]
            .local_transform_mut()
            .set_position(self.weapon_offset);

        let camera_node = &mut context.scene.graph[self.camera];
        camera_node
            .local_transform_mut()
            .set_position(self.camera_offset);

        self.head_position = camera_node.global_position();
        self.look_direction = camera_node.look_vector();
        self.up_direction = camera_node.up_vector();
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
            let initial_velocity = context.scene.graph[self.character.body]
                .as_rigid_body()
                .lin_vel();

            if self.controller.shoot {
                self.character
                    .sender
                    .as_ref()
                    .unwrap()
                    .send(Message::ShootWeapon {
                        weapon: *current_weapon_handle,
                        initial_velocity,
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
                    position: self.character.position(&context.scene.graph),
                    gain: 1.0,
                    rolloff_factor: 2.0,
                    radius: 3.0,
                })
                .unwrap();

            self.path_len = 0.0;
        }
    }

    pub fn clean_up(&mut self, scene: &mut Scene) {
        self.character.clean_up(scene)
    }
}
