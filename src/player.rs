use rg3d::{
    WindowEvent,
    MouseButton,
    MouseScrollDelta,
    ElementState,
    VirtualKeyCode,
    engine::{
        resource_manager::ResourceManager
    },
    scene::{
        SceneInterfaceMut,
        node::{Node, NodeTrait},
        Scene,
        graph::Graph,
    },
};
use crate::{
    weapon::Weapon,
    GameTime,
    projectile::ProjectileContainer,
};
use rg3d_core::{
    visitor::{Visit, Visitor, VisitResult},
    pool::Handle,
    math::{vec2::Vec2, vec3::Vec3, quat::Quat},
};
use rg3d_sound::{
    source::{Source, SourceKind},
    buffer::BufferKind,
    context::Context,
};
use std::{
    path::Path,
    sync::{Arc, Mutex},
};
use rand::Rng;
use rg3d_physics::{
    convex_shape::ConvexShape,
    rigid_body::RigidBody,
    Physics,
    convex_shape::{CapsuleShape, Axis},
};
use crate::actor::ActorTrait;

pub struct Controller {
    move_forward: bool,
    move_backward: bool,
    move_left: bool,
    move_right: bool,
    crouch: bool,
    jump: bool,
    run: bool,
    last_mouse_pos: Vec2,
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
            last_mouse_pos: Vec2::ZERO,
            shoot: false,
        }
    }
}

pub struct Player {
    pivot: Handle<Node>,
    body: Handle<RigidBody>,
    health: f32,
    camera: Handle<Node>,
    camera_pivot: Handle<Node>,
    weapon_pivot: Handle<Node>,
    controller: Controller,
    yaw: f32,
    dest_yaw: f32,
    pitch: f32,
    dest_pitch: f32,
    run_speed_multiplier: f32,
    stand_body_radius: f32,
    crouch_body_radius: f32,
    move_speed: f32,
    weapons: Vec<Weapon>,
    camera_offset: Vec3,
    camera_dest_offset: Vec3,
    current_weapon: u32,
    footsteps: Vec<Handle<Source>>,
    path_len: f32,
    feet_position: Vec3,
    head_position: Vec3,
    look_direction: Vec3,
    up_direction: Vec3,
}

impl ActorTrait for Player {
    fn get_body(&self) -> Handle<RigidBody> { self.body }
    fn get_health(&self) -> f32 { self.health }
    fn set_health(&mut self, health: f32) { self.health = health; }
    fn remove_self(&self, scene: &mut Scene) {}

    fn update(&mut self, sound_context: Arc<Mutex<Context>>, resource_manager: &mut ResourceManager, scene: &mut Scene, time: &GameTime, projectiles: &mut ProjectileContainer) {
        self.update_movement(scene, time);

        if let Some(current_weapon) = self.weapons.get_mut(self.current_weapon as usize) {
            current_weapon.update(scene);

            if self.controller.shoot {
                current_weapon.shoot(resource_manager, scene, sound_context.clone(), time, projectiles);
            }
        }

        if self.path_len > 2.0 {
            self.emit_footstep_sound(sound_context.clone());
            self.path_len = 0.0;
        }

        self.update_listener(sound_context);
    }
}

impl Default for Player {
    fn default() -> Self {
        Self {
            camera: Default::default(),
            pivot: Default::default(),
            camera_pivot: Default::default(),
            controller: Controller::default(),
            stand_body_radius: 0.5,
            dest_pitch: 0.0,
            dest_yaw: 0.0,
            move_speed: 0.058,
            body: Default::default(),
            run_speed_multiplier: 1.75,
            crouch_body_radius: 0.35,
            yaw: 0.0,
            pitch: 0.0,
            weapons: Vec::new(),
            camera_dest_offset: Vec3::ZERO,
            camera_offset: Vec3::ZERO,
            weapon_pivot: Default::default(),
            current_weapon: 0,
            footsteps: Vec::new(),
            path_len: 0.0,
            feet_position: Vec3::ZERO,
            head_position: Vec3::ZERO,
            look_direction: Vec3::ZERO,
            up_direction: Vec3::ZERO,
            health: 100.0,
        }
    }
}

impl Visit for Player {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.camera.visit("Camera", visitor)?;
        self.camera_pivot.visit("CameraPivot", visitor)?;
        self.pivot.visit("Pivot", visitor)?;
        self.body.visit("Body", visitor)?;
        self.weapon_pivot.visit("WeaponPivot", visitor)?;
        self.yaw.visit("Yaw", visitor)?;
        self.dest_yaw.visit("DestYaw", visitor)?;
        self.pitch.visit("Pitch", visitor)?;
        self.dest_pitch.visit("DestPitch", visitor)?;
        self.run_speed_multiplier.visit("RunSpeedMultiplier", visitor)?;
        self.stand_body_radius.visit("StandBodyRadius", visitor)?;
        self.crouch_body_radius.visit("CrouchBodyRadius", visitor)?;
        self.move_speed.visit("MoveSpeed", visitor)?;
        self.weapons.visit("Weapons", visitor)?;
        self.camera_offset.visit("CameraOffset", visitor)?;
        self.camera_dest_offset.visit("CameraDestOffset", visitor)?;
        self.current_weapon.visit("CurrentWeapon", visitor)?;
        self.footsteps.visit("Footsteps", visitor)?;
        self.health.visit("Health", visitor)?;

        visitor.leave_region()
    }
}

impl Player {
    pub fn new(sound_context: Arc<Mutex<Context>>, resource_manager: &mut ResourceManager, scene: &mut Scene) -> Player {
        let SceneInterfaceMut { graph, physics, node_rigid_body_map, .. } = scene.interface_mut();

        let camera_handle = graph.add_node(Node::Camera(Default::default()));

        let mut camera_pivot = Node::Pivot(Default::default());
        camera_pivot.get_local_transform_mut().set_position(Vec3 { x: 0.0, y: 1.0, z: 0.0 });
        let camera_pivot_handle = graph.add_node(camera_pivot);
        graph.link_nodes(camera_handle, camera_pivot_handle);

        let mut pivot = Node::Pivot(Default::default());
        pivot.get_local_transform_mut().set_position(Vec3 { x: -1.0, y: 0.0, z: 1.0 });

        let stand_body_radius = 0.35;
        let body = RigidBody::new(ConvexShape::Capsule(CapsuleShape::new(stand_body_radius, 1.0, Axis::Y)));
        let body_handle = physics.add_body(body);
        let pivot_handle = graph.add_node(pivot);
        node_rigid_body_map.insert(pivot_handle, body_handle);
        graph.link_nodes(camera_pivot_handle, pivot_handle);

        let mut weapon_pivot = Node::Pivot(Default::default());
        weapon_pivot.get_local_transform_mut().set_position(Vec3::new(-0.065, -0.052, 0.02));
        let weapon_pivot_handle = graph.add_node(weapon_pivot);
        graph.link_nodes(weapon_pivot_handle, camera_handle);

        let footsteps = {
            [
                resource_manager.request_sound_buffer(Path::new("data/sounds/footsteps/FootStep_shoe_stone_step1.wav"), BufferKind::Normal).unwrap(),
                resource_manager.request_sound_buffer(Path::new("data/sounds/footsteps/FootStep_shoe_stone_step2.wav"), BufferKind::Normal).unwrap(),
                resource_manager.request_sound_buffer(Path::new("data/sounds/footsteps/FootStep_shoe_stone_step3.wav"), BufferKind::Normal).unwrap(),
                resource_manager.request_sound_buffer(Path::new("data/sounds/footsteps/FootStep_shoe_stone_step4.wav"), BufferKind::Normal).unwrap()
            ]
        };

        Player {
            camera: camera_handle,
            pivot: pivot_handle,
            camera_pivot: camera_pivot_handle,
            stand_body_radius,
            body: body_handle,
            weapon_pivot: weapon_pivot_handle,
            footsteps: {
                let mut sound_context = sound_context.lock().unwrap();
                footsteps.iter().map(|buf| {
                    let source = Source::new_spatial(buf.clone()).unwrap();
                    sound_context.add_source(source)
                }).collect()
            },
            .. Default::default()
        }
    }

    pub fn add_weapon(&mut self, graph: &mut Graph, weapon: Weapon) {
        graph.link_nodes(weapon.get_model(), self.weapon_pivot);
        self.weapons.push(weapon);
    }

    pub fn next_weapon(&mut self) {
        if !self.weapons.is_empty() && (self.current_weapon as usize) < self.weapons.len() - 1 {
            self.current_weapon += 1;
        }
    }

    pub fn prev_weapon(&mut self) {
        if self.current_weapon > 0 {
            self.current_weapon -= 1;
        }
    }

    pub fn has_ground_contact(&self, physics: &Physics) -> bool {
        let body = physics.borrow_body(self.body);
        for contact in body.get_contacts() {
            if contact.normal.y >= 0.7 {
                return true;
            }
        }
        false
    }

    fn update_movement(&mut self, scene: &mut Scene, time: &GameTime) {
        let SceneInterfaceMut { graph, physics, .. } = scene.interface_mut();

        let (look, side) = {
            let pivot_node = graph.get(self.pivot);
            (pivot_node.get_look_vector(), pivot_node.get_side_vector())
        };

        let has_ground_contact = self.has_ground_contact(physics);

        let mut is_moving = false;
        let body = physics.borrow_body_mut(self.body);
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

        let speed_mult =
            if self.controller.run {
                self.run_speed_multiplier
            } else {
                1.0
            };

        if let Some(normalized_velocity) = velocity.normalized() {
            body.set_x_velocity(normalized_velocity.x * self.move_speed * speed_mult);
            body.set_z_velocity(normalized_velocity.z * self.move_speed * speed_mult);

            is_moving = true;
        }

        if self.controller.jump {
            if has_ground_contact {
                body.set_y_velocity(0.07);
            }
            self.controller.jump = false;
        }

        self.feet_position = body.get_position();

        if let ConvexShape::Sphere(sphere) = body.get_shape() {
            self.feet_position.y -= sphere.get_radius();
        }

        if has_ground_contact && is_moving {
            let k = (time.elapsed * 15.0) as f32;
            self.camera_dest_offset.x = 0.05 * (k * 0.5).cos();
            self.camera_dest_offset.y = 0.1 * k.sin();
            self.path_len += 0.1;
        }

        self.camera_offset.x += (self.camera_dest_offset.x - self.camera_offset.x) * 0.1;
        self.camera_offset.y += (self.camera_dest_offset.y - self.camera_offset.y) * 0.1;
        self.camera_offset.z += (self.camera_dest_offset.z - self.camera_offset.z) * 0.1;

        {
            let camera_node = graph.get_mut(self.camera);
            camera_node.get_local_transform_mut().set_position(self.camera_offset);

            self.head_position = camera_node.get_global_position();
            self.look_direction = camera_node.get_look_vector();
            self.up_direction = camera_node.get_up_vector();
        }

        for (i, weapon) in self.weapons.iter().enumerate() {
            weapon.set_visibility(i == self.current_weapon as usize, graph);
        }

        self.yaw += (self.dest_yaw - self.yaw) * 0.2;
        self.pitch += (self.dest_pitch - self.pitch) * 0.2;

        graph.get_mut(self.pivot).get_local_transform_mut().set_rotation(Quat::from_axis_angle(Vec3::UP, self.yaw.to_radians()));
        graph.get_mut(self.camera_pivot).get_local_transform_mut().set_rotation(Quat::from_axis_angle(Vec3::RIGHT, self.pitch.to_radians()));
    }

    fn emit_footstep_sound(&self, sound_context: Arc<Mutex<Context>>) {
        let mut sound_context = sound_context.lock().unwrap();
        let handle = self.footsteps[rand::thread_rng().gen_range(0, self.footsteps.len())];
        let source = sound_context.get_source_mut(handle);
        if let SourceKind::Spatial(spatial) = source.get_kind_mut() {
            spatial.set_position(&self.feet_position);
        }
        source.play();
    }

    pub fn set_position(&mut self, physics: &mut Physics, position: Vec3) {
        physics.borrow_body_mut(self.body).set_position(position);
    }

    pub fn get_position(&self, physics: &Physics) -> Vec3 {
        physics.borrow_body(self.body).get_position()
    }

    fn update_listener(&mut self, sound_context: Arc<Mutex<Context>>) {
        let mut sound_context = sound_context.lock().unwrap();
        let listener = sound_context.get_listener_mut();
        listener.set_position(&self.head_position);
        listener.set_orientation(&self.look_direction, &self.up_direction).unwrap();
    }



    pub fn process_input_event(&mut self, event: &WindowEvent) -> bool {
        match event {
            WindowEvent::CursorMoved { position, .. } => {
                let mouse_velocity = Vec2 {
                    x: position.x as f32 - self.controller.last_mouse_pos.x,
                    y: position.y as f32 - self.controller.last_mouse_pos.y,
                };

                let sens: f32 = 0.3;

                self.dest_pitch += mouse_velocity.y * sens;
                self.dest_yaw -= mouse_velocity.x * sens;

                if self.dest_pitch > 90.0 {
                    self.dest_pitch = 90.0;
                } else if self.dest_pitch < -90.0 {
                    self.dest_pitch = -90.0;
                }

                self.controller.last_mouse_pos = Vec2 {
                    x: position.x as f32,
                    y: position.y as f32,
                };
            }

            WindowEvent::MouseInput { button, state, .. } => {
                if let MouseButton::Left = button {
                    match state {
                        ElementState::Pressed => {
                            self.controller.shoot = true;
                        }
                        ElementState::Released => {
                            self.controller.shoot = false;
                        }
                    }
                }
            }

            WindowEvent::MouseWheel { delta, .. } => {
                if let MouseScrollDelta::LineDelta(_, y) = delta {
                    if *y < 0.0 {
                        self.prev_weapon();
                    } else if *y > 0.0 {
                        self.next_weapon();
                    }
                }
            }

            WindowEvent::KeyboardInput { input, .. } => {
                match input.state {
                    ElementState::Pressed => {
                        if let Some(key) = input.virtual_keycode {
                            match key {
                                VirtualKeyCode::W => self.controller.move_forward = true,
                                VirtualKeyCode::S => self.controller.move_backward = true,
                                VirtualKeyCode::A => self.controller.move_left = true,
                                VirtualKeyCode::D => self.controller.move_right = true,
                                VirtualKeyCode::C => self.controller.crouch = true,
                                VirtualKeyCode::Space => self.controller.jump = true,
                                VirtualKeyCode::LShift => self.controller.run = true,
                                _ => ()
                            }
                        }
                    }
                    ElementState::Released => {
                        if let Some(key) = input.virtual_keycode {
                            match key {
                                VirtualKeyCode::W => self.controller.move_forward = false,
                                VirtualKeyCode::S => self.controller.move_backward = false,
                                VirtualKeyCode::A => self.controller.move_left = false,
                                VirtualKeyCode::D => self.controller.move_right = false,
                                VirtualKeyCode::C => self.controller.crouch = false,
                                VirtualKeyCode::LShift => self.controller.run = false,
                                _ => ()
                            }
                        }
                    }
                }
            }
            _ => ()
        }
        false
    }
}

