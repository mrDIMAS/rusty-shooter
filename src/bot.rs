use std::{
    path::Path,
    cell::Cell,
    sync::mpsc::Sender,
};
use crate::{
    character::{Character, AsCharacter},
    level::{
        LevelEntity,
        CleanUp,
        LevelUpdateContext,
    },
    message::Message,
    actor::Actor,
    GameTime,
    actor::TargetDescriptor,
    item::ItemContainer,
};
use rg3d::{
    core::{
        pool::Handle,
        visitor::{
            Visit,
            VisitResult,
            Visitor,
        },
        math::{
            vec3::Vec3,
            mat4::Mat4,
            quat::Quat,
            frustum::Frustum,
        },
        color::Color,
    },
    physics::{
        rigid_body::RigidBody,
        convex_shape::{ConvexShape, CapsuleShape, Axis},
    },
    animation::{
        Animation,
        machine::{
            self,
            Machine,
            State,
            PoseNode,
        },
    },
    scene::{
        node::Node,
        Scene,
        base::{AsBase, BaseBuilder},
        transform::TransformBuilder,
        graph::Graph,
    },
    engine::resource_manager::ResourceManager,
    renderer::debug_renderer::{self, DebugRenderer},
    animation::AnimationSignal,
};
use rand::Rng;

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub enum BotKind {
    // Beasts
    Mutant,
    Parasite,
    Maw,
    // Humans
}

impl BotKind {
    pub fn new(id: i32) -> Result<Self, String> {
        match id {
            0 => Ok(BotKind::Mutant),
            1 => Ok(BotKind::Parasite),
            2 => Ok(BotKind::Maw),
            _ => Err(format!("Invalid bot kind {}", id))
        }
    }

    pub fn id(&self) -> i32 {
        match self {
            BotKind::Mutant => 0,
            BotKind::Parasite => 1,
            BotKind::Maw => 2,
        }
    }
}

pub struct Bot {
    target: Vec3,
    kind: BotKind,
    model: Handle<Node>,
    character: Character,
    target_actor: Cell<Handle<Actor>>,
    pub definition: &'static BotDefinition,
    locomotion_machine: LocomotionMachine,
    combat_machine: CombatMachine,
    dying_machine: DyingMachine,
    last_health: f32,
    restoration_time: f32,
    shoot_interval: f32,
    shots_made: usize,
    path: Vec<Vec3>,
    move_target: Vec3,
    current_path_point: usize,
    frustum: Frustum,
    last_poi_update_time: f64,
    point_of_interest: Vec3,
    last_path_rebuild_time: f64,
    last_move_dir: Vec3,
    spine: Handle<Node>,
}

impl AsCharacter for Bot {
    fn character(&self) -> &Character {
        &self.character
    }

    fn character_mut(&mut self) -> &mut Character {
        &mut self.character
    }
}

impl Default for Bot {
    fn default() -> Self {
        Self {
            character: Default::default(),
            kind: BotKind::Mutant,
            model: Default::default(),
            target: Default::default(),
            target_actor: Default::default(),
            definition: Self::get_definition(BotKind::Mutant),
            locomotion_machine: Default::default(),
            combat_machine: Default::default(),
            dying_machine: Default::default(),
            last_health: 0.0,
            restoration_time: 0.0,
            shoot_interval: 0.0,
            shots_made: 0,
            path: Default::default(),
            move_target: Default::default(),
            current_path_point: 0,
            frustum: Default::default(),
            last_poi_update_time: -10.0,
            point_of_interest: Default::default(),
            last_path_rebuild_time: -10.0,
            last_move_dir: Default::default(),
            spine: Default::default(),
        }
    }
}

pub struct BotDefinition {
    pub scale: f32,
    pub health: f32,
    pub kind: BotKind,
    pub walk_speed: f32,
    pub weapon_scale: f32,
    pub model: &'static str,
    pub idle_animation: &'static str,
    pub walk_animation: &'static str,
    pub aim_animation: &'static str,
    pub whip_animation: &'static str,
    pub jump_animation: &'static str,
    pub falling_animation: &'static str,
    pub hit_reaction_animation: &'static str,
    pub dying_animation: &'static str,
    pub dead_animation: &'static str,
    pub weapon_hand_name: &'static str,
    pub left_leg_name: &'static str,
    pub right_leg_name: &'static str,
    pub spine: &'static str,
    pub v_aim_angle_hack: f32
}

impl LevelEntity for Bot {
    fn update(&mut self, context: &mut LevelUpdateContext) {
        if self.character.is_dead() {
            self.dying_machine.machine
                .set_parameter(DYING_TO_DEAD, machine::Parameter::Rule(self.character.is_dead()))
                .evaluate_pose(&context.scene.animations, context.time.delta)
                .apply(&mut context.scene.graph);
        } else {
            self.select_point_of_interest(context.items, context.scene, &context.time);

            let threshold = 2.0;
            let has_ground_contact = self.character.has_ground_contact(&context.scene.physics);
            let body = context.scene.physics.borrow_body_mut(self.character.body);
            let look_dir = self.target - body.get_position();
            let distance = look_dir.len();

            let position = body.get_position();

            if let Some(path_point) = self.path.get(self.current_path_point) {
                self.move_target = *path_point;
                if self.move_target.distance(&position) <= 2.0 {
                    if self.current_path_point < self.path.len() - 1 {
                        self.current_path_point += 1;
                    }
                }
            }

            let head_pos = position + Vec3::new(0.0, 0.8, 0.0);
            let up = context.scene.graph.get(self.model).base().get_up_vector();
            let look_at = head_pos + context.scene.graph.get(self.model).base().get_look_vector();
            let view_matrix = Mat4::look_at(head_pos, look_at, up).unwrap_or_default();
            let projection_matrix = Mat4::perspective(60.0f32.to_radians(), 16.0 / 9.0, 0.1, 7.0);
            let view_projection_matrix = projection_matrix * view_matrix;
            self.frustum = Frustum::from(view_projection_matrix).unwrap();

            if let Some(look_dir) = look_dir.normalized() {
                let v_aim_angle = look_dir.dot(&Vec3::UP).acos() - std::f32::consts::PI / 2.0 + self.definition.v_aim_angle_hack.to_radians();
                if self.spine.is_some() {
                    context.scene
                        .graph
                        .get_mut(self.spine)
                        .base_mut()
                        .get_local_transform_mut()
                        .set_rotation(Quat::from_axis_angle(Vec3::RIGHT, v_aim_angle));
                }

                if distance > threshold {
                    if has_ground_contact {
                        if let Some(move_dir) = (self.move_target - position).normalized() {
                            let vel = move_dir.scale(self.definition.walk_speed * context.time.delta);
                            body.set_x_velocity(vel.x);
                            body.set_z_velocity(vel.z);
                            self.last_move_dir = move_dir;
                        }
                    } else {
                        // A bit of air control. This helps jump of ledges when there is jump pad below bot.
                        let vel = self.last_move_dir.scale(self.definition.walk_speed * context.time.delta);
                        body.set_x_velocity(vel.x);
                        body.set_z_velocity(vel.z);
                    }
                }

                let pivot = context.scene.graph.get_mut(self.character.pivot);
                let transform = pivot.base_mut().get_local_transform_mut();
                let angle = look_dir.x.atan2(look_dir.z);
                transform.set_rotation(Quat::from_axis_angle(Vec3::UP, angle))
            }

            let need_jump = look_dir.y >= 0.3 && has_ground_contact && distance < 2.0;
            if need_jump {
                body.set_y_velocity(0.08);
            }
            let was_damaged = self.character.health < self.last_health;
            if was_damaged {
                let hit_reaction = context.scene.animations.get_mut(self.combat_machine.hit_reaction_animation);
                if hit_reaction.has_ended() {
                    hit_reaction.rewind();
                }
                self.restoration_time = 0.8;
            }
            let can_aim = self.restoration_time <= 0.0;
            self.last_health = self.character.health;

            self.locomotion_machine.machine
                .set_parameter(IDLE_TO_WALK_PARAM, machine::Parameter::Rule(distance > threshold))
                .set_parameter(WALK_TO_IDLE_PARAM, machine::Parameter::Rule(distance <= threshold))
                .set_parameter(WALK_TO_JUMP_PARAM, machine::Parameter::Rule(need_jump))
                .set_parameter(IDLE_TO_JUMP_PARAM, machine::Parameter::Rule(need_jump))
                .set_parameter(JUMP_TO_FALLING_PARAM, machine::Parameter::Rule(!has_ground_contact))
                .set_parameter(FALLING_TO_IDLE_PARAM, machine::Parameter::Rule(has_ground_contact))
                .evaluate_pose(&context.scene.animations, context.time.delta)
                .apply(&mut context.scene.graph);

            // Overwrite upper body with combat machine
            self.combat_machine.machine
                .set_parameter(WHIP_TO_AIM_PARAM, machine::Parameter::Rule(distance > threshold))
                .set_parameter(AIM_TO_WHIP_PARAM, machine::Parameter::Rule(distance <= threshold))
                .set_parameter(WHIP_TO_HIT_REACTION_PARAM, machine::Parameter::Rule(was_damaged))
                .set_parameter(AIM_TO_HIT_REACTION_PARAM, machine::Parameter::Rule(was_damaged))
                .set_parameter(HIT_REACTION_TO_AIM_PARAM, machine::Parameter::Rule(can_aim))
                .evaluate_pose(&context.scene.animations, context.time.delta)
                .apply(&mut context.scene.graph);

            self.shoot_interval -= context.time.delta;

            if distance > threshold && can_aim && self.can_shoot() {
                if let Some(weapon) = self.character.weapons.get(self.character.current_weapon as usize) {
                    self.character
                        .sender
                        .as_ref()
                        .unwrap()
                        .send(Message::ShootWeapon {
                            weapon: *weapon,
                            initial_velocity: Vec3::ZERO,
                        }).unwrap();
                    self.shots_made += 1;
                    if self.shots_made >= 4 {
                        self.shots_made = 0;
                        self.shoot_interval = 0.5;
                    }
                }
            }

            // Apply damage to target from melee attack
            while let Some(event) = context.scene.animations.get_mut(self.combat_machine.whip_animation).pop_event() {
                if event.signal_id == CombatMachine::HIT_SIGNAL && distance < threshold {
                    if self.target_actor.get().is_some() {
                        self.character
                            .sender
                            .as_ref()
                            .unwrap()
                            .send(Message::DamageActor {
                                actor: self.target_actor.get(),
                                who: Default::default(),
                                amount: 20.0,
                            })
                            .unwrap();
                    }
                }
            }

            // Emit step sounds from walking animation.
            if self.locomotion_machine.is_walking() {
                while let Some(event) = context.scene.animations.get_mut(self.locomotion_machine.walk_animation).pop_event() {
                    if event.signal_id == LocomotionMachine::STEP_SIGNAL && has_ground_contact {
                        let footsteps = [
                            "data/sounds/footsteps/FootStep_shoe_stone_step1.wav",
                            "data/sounds/footsteps/FootStep_shoe_stone_step2.wav",
                            "data/sounds/footsteps/FootStep_shoe_stone_step3.wav",
                            "data/sounds/footsteps/FootStep_shoe_stone_step4.wav"
                        ];
                        self.character
                            .sender
                            .as_ref()
                            .unwrap()
                            .send(Message::PlaySound {
                                path: footsteps[rand::thread_rng().gen_range(0, footsteps.len())].into(),
                                position,
                            })
                            .unwrap();
                    }
                }
            }

            if context.time.elapsed - self.last_path_rebuild_time >= 1.0 {
                if let Some(navmesh) = context.navmesh.as_mut() {
                    let from = body.get_position() - Vec3::new(0.0, 1.0, 0.0);
                    if let Some(from_index) = navmesh.query_closest(from) {
                        if let Some(to_index) = navmesh.query_closest(self.point_of_interest) {
                            self.current_path_point = 0;
                            // Rebuild path if target path vertex has changed.
                            if navmesh.build_path(from_index, to_index, &mut self.path).is_ok() {
                                self.path.reverse();
                                self.last_path_rebuild_time = context.time.elapsed;
                            }
                        }
                    }
                }
            }
            self.restoration_time -= context.time.delta;
        }
    }
}

fn load_animation<P: AsRef<Path>>(
    resource_manager: &mut ResourceManager,
    path: P,
    model: Handle<Node>,
    scene: &mut Scene,
    spine: Handle<Node>
) -> Result<Handle<Animation>, ()> {
    let animation = *resource_manager.request_model(path)
        .ok_or(())?
        .lock()
        .unwrap()
        .retarget_animations(model, scene)
        .get(0)
        .ok_or(())?;

    // Disable spine animation because it is used to control vertical aim.
    scene.animations
        .get_mut(animation)
        .set_node_track_enabled(spine, false);

    Ok(animation)
}

fn disable_leg_tracks(animation: &mut Animation, root: Handle<Node>, leg_name: &str, graph: &Graph) {
    animation.set_tracks_enabled_from(graph.find_by_name(root, leg_name), false, graph)
}

// Locomotion machine parameters
pub const WALK_TO_IDLE_PARAM: &'static str = "WalkToIdle";
pub const WALK_TO_JUMP_PARAM: &'static str = "WalkToJump";
pub const IDLE_TO_WALK_PARAM: &'static str = "IdleToWalk";
pub const IDLE_TO_JUMP_PARAM: &'static str = "IdleToJump";
pub const JUMP_TO_FALLING_PARAM: &'static str = "JumpToFalling";
pub const FALLING_TO_IDLE_PARAM: &'static str = "FallingToIdle";

// Combat machine parameters
pub const AIM_TO_WHIP_PARAM: &'static str = "AimToWhip";
pub const WHIP_TO_AIM_PARAM: &'static str = "WhipToAim";
pub const HIT_REACTION_TO_AIM_PARAM: &'static str = "HitReactionToAim";
pub const AIM_TO_HIT_REACTION_PARAM: &'static str = "AimToHitReaction";
pub const WHIP_TO_HIT_REACTION_PARAM: &'static str = "WhipToHitReaction";

// Dying machine parameters
pub const DYING_TO_DEAD: &'static str = "DyingToDead";

struct LocomotionMachine {
    machine: Machine,
    walk_animation: Handle<Animation>,
    walk_state: Handle<State>,
}

impl Default for LocomotionMachine {
    fn default() -> Self {
        Self {
            machine: Default::default(),
            walk_animation: Default::default(),
            walk_state: Default::default(),
        }
    }
}

impl Visit for LocomotionMachine {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.machine.visit("Machine", visitor)?;
        self.walk_animation.visit("WalkAnimation", visitor)?;
        self.walk_state.visit("WalkState", visitor)?;

        visitor.leave_region()
    }
}

impl LocomotionMachine {
    pub const STEP_SIGNAL: u64 = 1;

    fn new(
        resource_manager: &mut ResourceManager,
        definition: &BotDefinition,
        model: Handle<Node>,
        scene: &mut Scene,
        spine: Handle<Node>
    ) -> Result<Self, ()> {
        let idle_animation = load_animation(resource_manager, definition.idle_animation, model, scene, spine)?;

        let walk_animation = load_animation(resource_manager, definition.walk_animation, model, scene, spine)?;
        scene.animations
            .get_mut(walk_animation)
            .add_signal(AnimationSignal::new(Self::STEP_SIGNAL, 0.4))
            .add_signal(AnimationSignal::new(Self::STEP_SIGNAL, 0.8));

        let jump_animation = load_animation(resource_manager, definition.jump_animation, model, scene, spine)?;
        let falling_animation = load_animation(resource_manager, definition.falling_animation, model, scene, spine)?;

        let mut machine = Machine::new();

        let jump_node = machine.add_node(machine::PoseNode::make_play_animation(jump_animation));
        let jump_state = machine.add_state(State::new("Jump", jump_node));

        let falling_node = machine.add_node(machine::PoseNode::make_play_animation(falling_animation));
        let falling_state = machine.add_state(State::new("Falling", falling_node));

        let walk_node = machine.add_node(machine::PoseNode::make_play_animation(walk_animation));
        let walk_state = machine.add_state(State::new("Walk", walk_node));

        let idle_node = machine.add_node(machine::PoseNode::make_play_animation(idle_animation));
        let idle_state = machine.add_state(State::new("Idle", idle_node));

        machine.add_transition(machine::Transition::new("Walk->Idle", walk_state, idle_state, 0.5, WALK_TO_IDLE_PARAM))
            .add_transition(machine::Transition::new("Walk->Jump", walk_state, jump_state, 0.5, WALK_TO_JUMP_PARAM))
            .add_transition(machine::Transition::new("Idle->Walk", idle_state, walk_state, 0.5, IDLE_TO_WALK_PARAM))
            .add_transition(machine::Transition::new("Idle->Jump", idle_state, jump_state, 0.5, IDLE_TO_JUMP_PARAM))
            .add_transition(machine::Transition::new("Jump->Falling", jump_state, falling_state, 0.5, JUMP_TO_FALLING_PARAM))
            .add_transition(machine::Transition::new("Falling->Idle", falling_state, idle_state, 0.5, FALLING_TO_IDLE_PARAM));

        machine.set_entry_state(idle_state);

        Ok(Self {
            walk_animation,
            walk_state,
            machine,
        })
    }

    pub fn is_walking(&self) -> bool {
        let active_transition = self.machine.active_transition();
        self.machine.active_state() == self.walk_state ||
            (active_transition.is_some() && self.machine.transitions().borrow(active_transition).dest() == self.walk_state)
    }
}

impl CleanUp for LocomotionMachine {
    fn clean_up(&mut self, scene: &mut Scene) {
        clean_machine(&self.machine, scene);
    }
}

struct DyingMachine {
    machine: Machine,
    dead_state: Handle<State>,
}

impl Default for DyingMachine {
    fn default() -> Self {
        Self {
            machine: Default::default(),
            dead_state: Default::default(),
        }
    }
}

impl DyingMachine {
    fn new(
        resource_manager: &mut ResourceManager,
        definition: &BotDefinition,
        model: Handle<Node>,
        scene: &mut Scene,
        spine: Handle<Node>
    ) -> Result<Self, ()> {
        let dying_animation = load_animation(resource_manager, definition.dying_animation, model, scene, spine)?;
        let dead_animation = load_animation(resource_manager, definition.dead_animation, model, scene, spine)?;

        let mut machine = Machine::new();

        let dying_node = machine.add_node(machine::PoseNode::make_play_animation(dying_animation));
        let dying_state = machine.add_state(State::new("Dying", dying_node));

        let dead_node = machine.add_node(machine::PoseNode::make_play_animation(dead_animation));
        let dead_state = machine.add_state(State::new("Dead", dead_node));

        machine.set_entry_state(dying_state);

        machine.add_transition(machine::Transition::new("Dying->Dead", dying_state, dead_state, 1.5, DYING_TO_DEAD));

        Ok(Self {
            machine,
            dead_state,
        })
    }
}

impl Visit for DyingMachine {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.machine.visit("Machine", visitor)?;
        self.dead_state.visit("DeadState", visitor)?;

        visitor.leave_region()
    }
}

impl CleanUp for DyingMachine {
    fn clean_up(&mut self, scene: &mut Scene) {
        clean_machine(&self.machine, scene);
    }
}

struct CombatMachine {
    machine: Machine,
    hit_reaction_animation: Handle<Animation>,
    whip_animation: Handle<Animation>,
    aim_state: Handle<State>,
}

impl Default for CombatMachine {
    fn default() -> Self {
        Self {
            machine: Default::default(),
            hit_reaction_animation: Default::default(),
            whip_animation: Default::default(),
            aim_state: Default::default(),
        }
    }
}

impl CombatMachine {
    pub const HIT_SIGNAL: u64 = 1;

    fn new(
        resource_manager: &mut ResourceManager,
        definition: &BotDefinition,
        model: Handle<Node>,
        scene: &mut Scene,
        spine: Handle<Node>
    ) -> Result<Self, ()> {
        let aim_animation = load_animation(resource_manager, definition.aim_animation, model, scene, spine)?;

        let whip_animation = load_animation(resource_manager, definition.whip_animation, model, scene, spine)?;
        scene.animations
            .get_mut(whip_animation)
            .add_signal(AnimationSignal::new(Self::HIT_SIGNAL, 0.9));

        let hit_reaction_animation = load_animation(resource_manager, definition.hit_reaction_animation, model, scene, spine)?;
        scene.animations
            .get_mut(hit_reaction_animation)
            .set_loop(false)
            .set_speed(2.0);

        // These animations must *not* affect legs, because legs animated using locomotion machine
        disable_leg_tracks(scene.animations.get_mut(aim_animation), model, definition.left_leg_name, &mut scene.graph);
        disable_leg_tracks(scene.animations.get_mut(aim_animation), model, definition.right_leg_name, &mut scene.graph);

        disable_leg_tracks(scene.animations.get_mut(whip_animation), model, definition.left_leg_name, &mut scene.graph);
        disable_leg_tracks(scene.animations.get_mut(whip_animation), model, definition.right_leg_name, &mut scene.graph);

        disable_leg_tracks(scene.animations.get_mut(hit_reaction_animation), model, definition.left_leg_name, &mut scene.graph);
        disable_leg_tracks(scene.animations.get_mut(hit_reaction_animation), model, definition.right_leg_name, &mut scene.graph);

        let mut machine = Machine::new();

        let hit_reaction_node = machine.add_node(machine::PoseNode::make_play_animation(hit_reaction_animation));
        let hit_reaction_state = machine.add_state(State::new("HitReaction", hit_reaction_node));

        let aim_node = machine.add_node(machine::PoseNode::make_play_animation(aim_animation));
        let aim_state = machine.add_state(State::new("Aim", aim_node));

        let whip_node = machine.add_node(machine::PoseNode::make_play_animation(whip_animation));
        let whip_state = machine.add_state(State::new("Whip", whip_node));

        machine.add_transition(machine::Transition::new("Aim->Whip", aim_state, whip_state, 0.5, AIM_TO_WHIP_PARAM))
            .add_transition(machine::Transition::new("Whip->Aim", whip_state, aim_state, 0.5, WHIP_TO_AIM_PARAM))
            .add_transition(machine::Transition::new("Whip->HitReaction", whip_state, hit_reaction_state, 0.2, WHIP_TO_HIT_REACTION_PARAM))
            .add_transition(machine::Transition::new("Aim->HitReaction", aim_state, hit_reaction_state, 0.2, AIM_TO_HIT_REACTION_PARAM))
            .add_transition(machine::Transition::new("HitReaction->Aim", hit_reaction_state, aim_state, 0.5, HIT_REACTION_TO_AIM_PARAM));

        Ok(Self {
            machine,
            hit_reaction_animation,
            whip_animation,
            aim_state,
        })
    }
}

impl CleanUp for CombatMachine {
    fn clean_up(&mut self, scene: &mut Scene) {
        clean_machine(&self.machine, scene)
    }
}

impl Visit for CombatMachine {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.machine.visit("Machine", visitor)?;
        self.hit_reaction_animation.visit("HitReactionAnimation", visitor)?;
        self.whip_animation.visit("WhipAnimation", visitor)?;
        self.aim_state.visit("AimState", visitor)?;

        visitor.leave_region()
    }
}

impl Bot {
    pub fn get_definition(kind: BotKind) -> &'static BotDefinition {
        match kind {
            BotKind::Mutant => {
                static DEFINITION: BotDefinition = BotDefinition {
                    kind: BotKind::Mutant,
                    model: "data/models/mutant.FBX",
                    idle_animation: "data/animations/mutant/idle.fbx",
                    walk_animation: "data/animations/mutant/walk.fbx",
                    aim_animation: "data/animations/mutant/aim.fbx",
                    whip_animation: "data/animations/mutant/whip.fbx",
                    jump_animation: "data/animations/mutant/jump.fbx",
                    falling_animation: "data/animations/mutant/falling.fbx",
                    dying_animation: "data/animations/mutant/dying.fbx",
                    dead_animation: "data/animations/mutant/dead.fbx",
                    hit_reaction_animation: "data/animations/mutant/hit_reaction.fbx",
                    weapon_hand_name: "Mutant:RightHand",
                    left_leg_name: "Mutant:LeftUpLeg",
                    right_leg_name: "Mutant:RightUpLeg",
                    spine: "Mutant:Spine",
                    walk_speed: 6.0,
                    scale: 0.0085,
                    weapon_scale: 2.6,
                    health: 100.0,
                    v_aim_angle_hack: -2.0,
                };
                &DEFINITION
            }
            BotKind::Parasite => {
                static DEFINITION: BotDefinition = BotDefinition {
                    kind: BotKind::Parasite,
                    model: "data/models/parasite.FBX",
                    idle_animation: "data/animations/parasite/idle.fbx",
                    walk_animation: "data/animations/parasite/walk.fbx",
                    aim_animation: "data/animations/parasite/aim.fbx",
                    whip_animation: "data/animations/parasite/whip.fbx",
                    jump_animation: "data/animations/parasite/jump.fbx",
                    falling_animation: "data/animations/parasite/falling.fbx",
                    dying_animation: "data/animations/parasite/dying.fbx",
                    dead_animation: "data/animations/parasite/dead.fbx",
                    hit_reaction_animation: "data/animations/parasite/hit_reaction.fbx",
                    weapon_hand_name: "RightHand",
                    left_leg_name: "LeftUpLeg",
                    right_leg_name: "RightUpLeg",
                    spine: "Spine",
                    walk_speed: 6.0,
                    scale: 0.0085,
                    weapon_scale: 2.5,
                    health: 100.0,
                    v_aim_angle_hack: 12.0
                };
                &DEFINITION
            }
            BotKind::Maw => {
                static DEFINITION: BotDefinition = BotDefinition {
                    kind: BotKind::Maw,
                    model: "data/models/maw.fbx",
                    idle_animation: "data/animations/maw/idle.fbx",
                    walk_animation: "data/animations/maw/walk.fbx",
                    aim_animation: "data/animations/maw/aim.fbx",
                    whip_animation: "data/animations/maw/whip.fbx",
                    jump_animation: "data/animations/maw/jump.fbx",
                    falling_animation: "data/animations/maw/falling.fbx",
                    dying_animation: "data/animations/maw/dying.fbx",
                    dead_animation: "data/animations/maw/dead.fbx",
                    hit_reaction_animation: "data/animations/maw/hit_reaction.fbx",
                    weapon_hand_name: "RightHand",
                    left_leg_name: "LeftUpLeg",
                    right_leg_name: "RightUpLeg",
                    spine: "Spine",
                    walk_speed: 6.0,
                    scale: 0.0085,
                    weapon_scale: 2.5,
                    health: 100.0,
                    v_aim_angle_hack: 16.0
                };
                &DEFINITION
            }
        }
    }

    pub fn new(kind: BotKind, resource_manager: &mut ResourceManager, scene: &mut Scene, position: Vec3, sender: Sender<Message>) -> Result<Self, ()> {
        let definition = Self::get_definition(kind);

        let body_height = 1.25;

        let model = resource_manager.request_model(Path::new(definition.model))
            .ok_or(())?
            .lock()
            .unwrap()
            .instantiate_geometry(scene);

        let spine = scene.graph.find_by_name(model, definition.spine);
        if spine.is_none() {
            print!("WARNING: Spine bone not found, bot won't aim vertically!");
        }

        let (pivot, body) = {
            let pivot = scene.graph.add_node(Node::Base(Default::default()));
            scene.graph.link_nodes(model, pivot);
            let transform = scene.graph.get_mut(model).base_mut().get_local_transform_mut();
            transform.set_position(Vec3::new(0.0, -body_height * 0.5, 0.0));
            transform.set_scale(Vec3::new(definition.scale, definition.scale, definition.scale));

            let capsule_shape = CapsuleShape::new(0.28, body_height, Axis::Y);
            let mut capsule_body = RigidBody::new(ConvexShape::Capsule(capsule_shape));
            capsule_body.set_friction(Vec3::new(0.2, 0.0, 0.2));
            capsule_body.set_position(position);
            let body = scene.physics.add_body(capsule_body);
            scene.physics_binder.bind(pivot, body);

            (pivot, body)
        };

        let hand = scene.graph.find_by_name(model, definition.weapon_hand_name);
        let wpn_scale = definition.weapon_scale * (1.0 / definition.scale);
        let weapon_pivot = Node::Base(BaseBuilder::new()
            .with_local_transform(TransformBuilder::new()
                .with_local_scale(Vec3::new(wpn_scale, wpn_scale, wpn_scale))
                .with_local_rotation(
                    Quat::from_axis_angle(Vec3::RIGHT, std::f32::consts::FRAC_PI_2) *
                        Quat::from_axis_angle(Vec3::UP, -std::f32::consts::FRAC_PI_2))
                .build())
            .build());
        let weapon_pivot = scene.graph.add_node(weapon_pivot);
        scene.graph.link_nodes(weapon_pivot, hand);

        let locomotion_machine = LocomotionMachine::new(resource_manager, &definition, model, scene, spine)?;
        let combat_machine = CombatMachine::new(resource_manager, definition, model, scene, spine)?;
        let dying_machine = DyingMachine::new(resource_manager, definition, model, scene, spine)?;

        Ok(Self {
            character: Character {
                pivot,
                body,
                weapon_pivot,
                health: definition.health,
                sender: Some(sender),
                name: format!("{:?}", kind),
                ..Default::default()
            },
            spine,
            definition,
            last_health: definition.health,
            model,
            kind,
            locomotion_machine,
            combat_machine,
            dying_machine,
            ..Default::default()
        })
    }

    pub fn can_be_removed(&self) -> bool {
        self.dying_machine.machine.active_state() == self.dying_machine.dead_state
    }

    pub fn can_shoot(&self) -> bool {
        self.combat_machine.machine.active_state() == self.combat_machine.aim_state
    }

    pub fn select_target(&mut self, self_handle: Handle<Actor>, scene: &Scene, target_descriptors: &[TargetDescriptor]) {
        let position = self.character.get_position(&scene.physics);
        let mut closest_distance = std::f32::MAX;
        for desc in target_descriptors {
            if desc.handle != self_handle {
                let sqr_d = position.sqr_distance(&desc.position);
                if sqr_d < closest_distance {
                    self.target = desc.position;
                    self.target_actor.set(desc.handle);
                    closest_distance = sqr_d;
                }
            }
        }
    }

    pub fn select_point_of_interest(&mut self, items: &ItemContainer, scene: &Scene, time: &GameTime) {
        if time.elapsed - self.last_poi_update_time >= 1.0 {
            // Select closest non-despawned item as point of interest.
            let self_position = self.character().get_position(&scene.physics);
            let mut closest_distance = std::f32::MAX;
            for item in items.iter() {
                if !item.is_picked_up() {
                    let item_position = item.position(&scene.graph);
                    let sqr_d = item_position.sqr_distance(&self_position);
                    if sqr_d < closest_distance {
                        closest_distance = sqr_d;
                        self.point_of_interest = item_position;
                    }
                }
            }
            self.last_poi_update_time = time.elapsed;
        }
    }

    pub fn debug_draw(&self, debug_renderer: &mut DebugRenderer) {
        for pts in self.path.windows(2) {
            let a = pts[0];
            let b = pts[1];
            debug_renderer.add_line(debug_renderer::Line {
                begin: a,
                end: b,
                color: Color::from_rgba(255, 0, 0, 255),
            });
        }

        debug_renderer.draw_frustum(&self.frustum, Color::from_rgba(0, 200, 0, 255));
    }
}

fn clean_machine(machine: &Machine, scene: &mut Scene) {
    for node in machine.nodes() {
        if let PoseNode::PlayAnimation(node) = node {
            scene.animations.remove(node.animation);
        }
    }
}

impl CleanUp for Bot {
    fn clean_up(&mut self, scene: &mut Scene) {
        self.combat_machine.clean_up(scene);
        self.dying_machine.clean_up(scene);
        self.locomotion_machine.clean_up(scene);
        self.character.clean_up(scene);
    }
}

impl Visit for Bot {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        let mut kind_id = self.kind.id();
        kind_id.visit("Kind", visitor)?;
        if visitor.is_reading() {
            self.kind = BotKind::new(kind_id)?;
        }

        self.definition = Self::get_definition(self.kind);
        self.character.visit("Character", visitor)?;
        self.model.visit("Model", visitor)?;
        self.target_actor.visit("TargetActor", visitor)?;
        self.locomotion_machine.visit("LocomotionMachine", visitor)?;
        self.combat_machine.visit("AimMachine", visitor)?;
        self.restoration_time.visit("RestorationTime", visitor)?;

        visitor.leave_region()
    }
}