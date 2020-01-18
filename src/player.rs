use rg3d::{
    core::{
        visitor::{Visit, Visitor, VisitResult},
        pool::Handle,
        math::{vec3::Vec3, quat::Quat, mat3::Mat3},
    },
    event::{DeviceEvent, Event, MouseScrollDelta, ElementState},
    engine::{
        resource_manager::ResourceManager
    },
    scene::{
        SceneInterfaceMut,
        node::Node,
        Scene,
        base::AsBase,
    },
    sound::{
        source::{
            SoundSource,
            spatial::SpatialSourceBuilder,
            generic::GenericSourceBuilder,
        },
        context::Context,
    },
    physics::{
        convex_shape::ConvexShape,
        rigid_body::RigidBody,
        Physics,
        convex_shape::{CapsuleShape, Axis},
    },
};
use rand::Rng;
use crate::{character::{
    AsCharacter,
    Character,
}, LevelUpdateContext, level::{
    LevelEntity,
    CleanUp,
}, ControlScheme, ControlButton};
use std::{
    rc::Rc,
    path::Path,
    sync::{Arc, Mutex},
    cell::RefCell,
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
    camera_offset: Vec3,
    camera_dest_offset: Vec3,
    footsteps: Vec<Handle<SoundSource>>,
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

impl AsCharacter for Player {
    fn character(&self) -> &Character {
        &self.character
    }

    fn character_mut(&mut self) -> &mut Character {
        &mut self.character
    }
}

impl LevelEntity for Player {
    fn update(&mut self, context: &mut LevelUpdateContext) {
        self.update_movement(context);

        if let Some(current_weapon) = self.character.weapons.get(self.character.current_weapon as usize) {
            let current_weapon = context.weapons.get_mut(*current_weapon);

            let velocity = context.scene.interface_mut().physics.borrow_body(self.character.body).get_velocity();

            if self.controller.shoot {
                if let Some(projectile) = current_weapon.try_shoot(context.scene, context.resource_manager,
                                                                   context.sound_context.clone(), context.time, velocity) {
                    context.projectiles.add(projectile);
                }
            }
        }

        if self.path_len > 2.0 {
            self.emit_footstep_sound(context.sound_context.clone());
            self.path_len = 0.0;
        }

        self.update_listener(context.sound_context.clone());
    }
}

impl Default for Player {
    fn default() -> Self {
        Self {
            character: Default::default(),
            camera: Default::default(),
            camera_pivot: Default::default(),
            controller: Controller::default(),
            stand_body_height: 0.85,
            dest_pitch: 0.0,
            dest_yaw: 0.0,
            move_speed: 0.058,
            run_speed_multiplier: 1.75,
            crouch_body_height: 0.15,
            yaw: 0.0,
            pitch: 0.0,
            camera_dest_offset: Vec3::ZERO,
            camera_offset: Vec3::ZERO,
            footsteps: Vec::new(),
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
        self.run_speed_multiplier.visit("RunSpeedMultiplier", visitor)?;
        self.stand_body_height.visit("StandBodyRadius", visitor)?;
        self.crouch_body_height.visit("CrouchBodyRadius", visitor)?;
        self.move_speed.visit("MoveSpeed", visitor)?;
        self.camera_offset.visit("CameraOffset", visitor)?;
        self.camera_dest_offset.visit("CameraDestOffset", visitor)?;
        self.footsteps.visit("Footsteps", visitor)?;

        visitor.leave_region()
    }
}

impl CleanUp for Player {
    fn clean_up(&mut self, scene: &mut Scene) {
        self.character.clean_up(scene)
    }
}

impl Player {
    pub fn new(sound_context: Arc<Mutex<Context>>, resource_manager: &mut ResourceManager, scene: &mut Scene) -> Player {
        let SceneInterfaceMut { graph, physics, node_rigid_body_map, .. } = scene.interface_mut();

        let camera_handle = graph.add_node(Node::Camera(Default::default()));

        let mut camera_pivot = Node::Base(Default::default());
        camera_pivot.base_mut().get_local_transform_mut().set_position(Vec3 { x: 0.0, y: 1.0, z: 0.0 });
        let camera_pivot_handle = graph.add_node(camera_pivot);
        graph.link_nodes(camera_handle, camera_pivot_handle);

        let mut pivot = Node::Base(Default::default());
        pivot.base_mut().get_local_transform_mut().set_position(Vec3 { x: -1.0, y: 0.0, z: 1.0 });

        let capsule_shape = CapsuleShape::new(0.35, Self::default().stand_body_height, Axis::Y);
        let body = RigidBody::new(ConvexShape::Capsule(capsule_shape));
        let body_handle = physics.add_body(body);
        let pivot_handle = graph.add_node(pivot);
        node_rigid_body_map.insert(pivot_handle, body_handle);
        graph.link_nodes(camera_pivot_handle, pivot_handle);

        let mut weapon_base_pivot = Node::Base(Default::default());
        weapon_base_pivot.base_mut().get_local_transform_mut().set_position(Vec3::new(-0.065, -0.052, 0.02));
        let weapon_base_pivot_handle = graph.add_node(weapon_base_pivot);
        graph.link_nodes(weapon_base_pivot_handle, camera_handle);

        let weapon_pivot = Node::Base(Default::default());
        let weapon_pivot_handle = graph.add_node(weapon_pivot);
        graph.link_nodes(weapon_pivot_handle, weapon_base_pivot_handle);

        let footsteps = {
            [
                resource_manager.request_sound_buffer(Path::new("data/sounds/footsteps/FootStep_shoe_stone_step1.wav"), false).unwrap(),
                resource_manager.request_sound_buffer(Path::new("data/sounds/footsteps/FootStep_shoe_stone_step2.wav"), false).unwrap(),
                resource_manager.request_sound_buffer(Path::new("data/sounds/footsteps/FootStep_shoe_stone_step3.wav"), false).unwrap(),
                resource_manager.request_sound_buffer(Path::new("data/sounds/footsteps/FootStep_shoe_stone_step4.wav"), false).unwrap()
            ]
        };

        Player {
            character: Character {
                pivot: pivot_handle,
                body: body_handle,
                weapon_pivot: weapon_pivot_handle,
                health: 999999.0,
                ..Default::default()
            },
            camera: camera_handle,
            camera_pivot: camera_pivot_handle,
            footsteps: {
                let mut sound_context = sound_context.lock().unwrap();
                footsteps.iter().map(|buf| {
                    let source = SpatialSourceBuilder::new(
                        GenericSourceBuilder::new(buf.clone())
                            .build()
                            .unwrap())
                        .build_source();
                    sound_context.add_source(source)
                }).collect()
            },
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

    pub fn set_control_scheme(&mut self, control_scheme: Rc<RefCell<ControlScheme>>) {
        self.control_scheme = Some(control_scheme);
    }

    fn update_movement(&mut self, context: &mut LevelUpdateContext) {
        let SceneInterfaceMut { graph, physics, .. } = context.scene.interface_mut();

        let pivot = graph.get(self.character.pivot).base();
        let look = pivot.get_look_vector();
        let side = pivot.get_side_vector();

        let has_ground_contact = self.character.has_ground_contact(physics);

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

        let speed_mult = if self.controller.run { self.run_speed_multiplier } else { 1.0 };

        let body = physics.borrow_body_mut(self.character.body);
        if let Some(normalized_velocity) = velocity.normalized() {
            body.set_x_velocity(normalized_velocity.x * self.move_speed * speed_mult);
            body.set_z_velocity(normalized_velocity.z * self.move_speed * speed_mult);

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
            self.weapon_dest_offset = Vec3::ZERO;
        }

        self.weapon_offset.follow(&self.weapon_dest_offset, 0.1);

        let weapon_pivot = graph.get_mut(self.character.weapon_pivot).base_mut();
        weapon_pivot.get_local_transform_mut().set_position(self.weapon_offset);

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

        let camera_node = graph.get_mut(self.camera).base_mut();
        camera_node.get_local_transform_mut().set_position(self.camera_offset);

        self.head_position = camera_node.get_global_position();
        self.look_direction = camera_node.get_look_vector();
        self.up_direction = camera_node.get_up_vector();
        self.listener_basis = Mat3::from_vectors(camera_node.get_side_vector(),
                                                 camera_node.get_up_vector(),
                                                 -camera_node.get_look_vector());

        for (i, weapon) in self.character.weapons.iter().enumerate() {
            let weapon = context.weapons.get_mut(*weapon);
            weapon.set_visibility(i == self.character.current_weapon as usize, graph);
        }

        if self.control_scheme.clone().unwrap().borrow().smooth_mouse {
            self.yaw += (self.dest_yaw - self.yaw) * 0.2;
            self.pitch += (self.dest_pitch - self.pitch) * 0.2;
        } else {
            self.yaw = self.dest_yaw;
            self.pitch = self.dest_pitch;
        }

        let pivot_transform = graph.get_mut(self.character.pivot).base_mut().get_local_transform_mut();
        pivot_transform.set_rotation(Quat::from_axis_angle(Vec3::UP, self.yaw.to_radians()));

        let camera_pivot_transform = graph.get_mut(self.camera_pivot).base_mut().get_local_transform_mut();
        camera_pivot_transform.set_rotation(Quat::from_axis_angle(Vec3::RIGHT, self.pitch.to_radians()));
    }

    fn emit_footstep_sound(&self, sound_context: Arc<Mutex<Context>>) {
        let mut sound_context = sound_context.lock().unwrap();
        let handle = self.footsteps[rand::thread_rng().gen_range(0, self.footsteps.len())];
        if let SoundSource::Spatial(spatial) = sound_context.source_mut(handle) {
            spatial.set_position(&self.feet_position);
            spatial.generic_mut().play();
        }
    }

    pub fn set_position(&mut self, physics: &mut Physics, position: Vec3) {
        physics.borrow_body_mut(self.character.body).set_position(position);
    }

    pub fn get_position(&self, physics: &Physics) -> Vec3 {
        physics.borrow_body(self.character.body).get_position()
    }

    fn update_listener(&mut self, sound_context: Arc<Mutex<Context>>) {
        let mut sound_context = sound_context.lock().unwrap();
        let listener = sound_context.listener_mut();
        listener.set_basis(self.listener_basis);
        listener.set_position(self.head_position);
    }

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

                    DeviceEvent::MouseWheel { delta } => {
                        if let MouseScrollDelta::LineDelta(_, y) = delta {
                            if *y < 0.0 {
                                self.character_mut().prev_weapon();
                            } else if *y > 0.0 {
                                self.character_mut().next_weapon();
                            }
                        }
                    }

                    DeviceEvent::Key(input) => {
                        if let Some(code) = input.virtual_keycode {
                            control_button = Some(ControlButton::Key(code));
                            control_button_state = input.state;
                        }
                    }
                    _ => ()
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
}
