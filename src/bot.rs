use std::{
    path::Path,
    cell::Cell,
    sync::mpsc::Sender
};
use crate::{
    character::{Character, AsCharacter},
    level::{LevelEntity, CleanUp, LevelUpdateContext},
    actor::Actor,
    level::GameEvent
};
use rg3d::{
    core::{
        pool::Handle,
        visitor::{Visit, VisitResult, Visitor},
        math::{vec3::Vec3, quat::Quat},
    },
    physics::{
        rigid_body::RigidBody,
        convex_shape::{ConvexShape, CapsuleShape, Axis},
    },
    animation::{
        Animation,
        machine,
        AnimationPose,
        machine::{Machine, State},
    },
    scene::{
        node::Node,
        Scene,
        SceneInterfaceMut,
        base::{AsBase, BaseBuilder},
        transform::TransformBuilder,
        graph::Graph,
    },
    engine::resource_manager::ResourceManager,
};

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
    hit_reaction_animation: Handle<Animation>,
    pub definition: &'static BotDefinition,
    pose: AnimationPose,
    locomotion_machine: Machine,
    combat_machine: Machine,
    dying_machine: Machine,
    last_health: f32,
    restoration_time: f32,
    dead_state: Handle<State>,
    aim_state: Handle<State>,
    shoot_interval: f32,
    shots_made: usize
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
            hit_reaction_animation: Default::default(),
            definition: Self::get_definition(BotKind::Mutant),
            pose: Default::default(),
            locomotion_machine: Default::default(),
            combat_machine: Default::default(),
            dying_machine: Default::default(),
            last_health: 0.0,
            restoration_time: 0.0,
            dead_state: Handle::NONE,
            aim_state: Handle::NONE,
            shoot_interval: 0.0,
            shots_made: 0,
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
}

impl LevelEntity for Bot {
    fn update(&mut self, context: &mut LevelUpdateContext) {
        let SceneInterfaceMut { graph, physics, animations, .. } = context.scene.interface_mut();

        if self.character.is_dead() {
            self.dying_machine
                .set_parameter(Self::DYING_TO_DEAD, machine::Parameter::Rule(self.character.is_dead()))
                .evaluate_pose(animations, context.time.delta)
                .apply(graph);
        } else {
            let threshold = 2.0;
            let has_ground_contact = self.character.has_ground_contact(physics);
            let body = physics.borrow_body_mut(self.character.body);
            let dir = self.target - body.get_position();
            let distance = dir.len();

            if let Some(dir) = dir.normalized() {
                if distance > threshold {
                    body.move_by(dir.scale(self.definition.walk_speed * context.time.delta));
                }

                let pivot = graph.get_mut(self.character.pivot);
                let transform = pivot.base_mut().get_local_transform_mut();
                let angle = dir.x.atan2(dir.z);
                transform.set_rotation(Quat::from_axis_angle(Vec3::UP, angle))
            }

            let need_jump = dir.y >= 0.3 && has_ground_contact;
            if need_jump {
                body.set_y_velocity(0.08);
            }
            let was_damaged = self.character.health < self.last_health;
            if was_damaged {
                let hit_reaction = animations.get_mut(self.hit_reaction_animation);
                if hit_reaction.has_ended() {
                    hit_reaction.rewind();
                }
                self.restoration_time = 0.8;
            }
            let can_aim = self.restoration_time <= 0.0;
            self.last_health = self.character.health;

            self.locomotion_machine
                .set_parameter(Self::IDLE_TO_WALK_PARAM, machine::Parameter::Rule(distance > threshold))
                .set_parameter(Self::WALK_TO_IDLE_PARAM, machine::Parameter::Rule(distance <= threshold))
                .set_parameter(Self::WALK_TO_JUMP_PARAM, machine::Parameter::Rule(need_jump))
                .set_parameter(Self::IDLE_TO_JUMP_PARAM, machine::Parameter::Rule(need_jump))
                .set_parameter(Self::JUMP_TO_FALLING_PARAM, machine::Parameter::Rule(!has_ground_contact))
                .set_parameter(Self::FALLING_TO_IDLE_PARAM, machine::Parameter::Rule(has_ground_contact))
                .evaluate_pose(animations, context.time.delta)
                .apply(graph);

            // Overwrite upper body with combat machine
            self.combat_machine
                .set_parameter(Self::WHIP_TO_AIM_PARAM, machine::Parameter::Rule(distance > threshold))
                .set_parameter(Self::AIM_TO_WHIP_PARAM, machine::Parameter::Rule(distance <= threshold))
                .set_parameter(Self::WHIP_TO_HIT_REACTION_PARAM, machine::Parameter::Rule(was_damaged))
                .set_parameter(Self::AIM_TO_HIT_REACTION_PARAM, machine::Parameter::Rule(was_damaged))
                .set_parameter(Self::HIT_REACTION_TO_AIM_PARAM, machine::Parameter::Rule(can_aim))
                .evaluate_pose(animations, context.time.delta)
                .apply(graph);

            self.shoot_interval -= context.time.delta;

            if distance > threshold && can_aim && self.can_shoot() {
                if let Some(weapon) = self.character.weapons.get(self.character.current_weapon as usize) {
                    self.character
                        .sender
                        .as_ref()
                        .unwrap()
                        .send(GameEvent::ShootWeapon {
                            weapon: *weapon,
                            initial_velocity: Vec3::ZERO
                        }).unwrap();
                    self.shots_made += 1;
                    if self.shots_made >= 4 {
                        self.shots_made = 0;
                        self.shoot_interval = 2.0;
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
    scene: &mut Scene
) -> Result<Handle<Animation>, ()> {
    Ok(*resource_manager.request_model(path)
        .ok_or(())?
        .lock()
        .unwrap()
        .retarget_animations(model, scene)
        .get(0)
        .ok_or(())?)
}

fn disable_leg_tracks(
    animation: &mut Animation,
    root: Handle<Node>,
    leg_name: &str,
    graph: &Graph
) {
    animation.set_tracks_enabled_from(graph.find_by_name(root, leg_name), false, graph)
}

impl Bot {
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
                    walk_speed: 0.3,
                    scale: 0.0085,
                    weapon_scale: 2.6,
                    health: 100.0,
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
                    walk_speed: 0.3,
                    scale: 0.0085,
                    weapon_scale: 2.5,
                    health: 100.0,
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
                    walk_speed: 0.3,
                    scale: 0.0085,
                    weapon_scale: 2.5,
                    health: 100.0,
                };
                &DEFINITION
            }
        }
    }

    pub fn new(kind: BotKind, resource_manager: &mut ResourceManager, scene: &mut Scene, position: Vec3, sender: Sender<GameEvent>) -> Result<Self, ()> {
        let definition = Self::get_definition(kind);

        let body_height = 1.25;

        let model = resource_manager.request_model(Path::new(definition.model))
            .ok_or(())?
            .lock()
            .unwrap()
            .instantiate_geometry(scene);

        let (pivot, body) = {
            let SceneInterfaceMut { graph, physics, node_rigid_body_map, .. } = scene.interface_mut();
            let pivot = graph.add_node(Node::Base(Default::default()));
            graph.link_nodes(model, pivot);
            let transform = graph.get_mut(model).base_mut().get_local_transform_mut();
            transform.set_position(Vec3::new(0.0, -body_height * 0.5, 0.0));
            transform.set_scale(Vec3::new(definition.scale, definition.scale, definition.scale));

            let capsule_shape = CapsuleShape::new(0.35, body_height, Axis::Y);
            let mut capsule_body = RigidBody::new(ConvexShape::Capsule(capsule_shape));
            capsule_body.set_position(position);
            let body = physics.add_body(capsule_body);
            node_rigid_body_map.insert(pivot, body);

            (pivot, body)
        };

        let hand = scene.interface().graph.find_by_name(model, definition.weapon_hand_name);
        let wpn_scale = definition.weapon_scale * (1.0 / definition.scale);
        let weapon_pivot = Node::Base(BaseBuilder::new()
            .with_local_transform(TransformBuilder::new()
                .with_local_scale(Vec3::new(wpn_scale, wpn_scale, wpn_scale))
                .with_local_rotation(
                    Quat::from_axis_angle(Vec3::RIGHT, std::f32::consts::FRAC_PI_2) *
                        Quat::from_axis_angle(Vec3::UP, -std::f32::consts::FRAC_PI_2))
                .build())
            .build());
        let graph = scene.interface_mut().graph;
        let weapon_pivot = graph.add_node(weapon_pivot);
        graph.link_nodes(weapon_pivot, hand);

        let locomotion_machine = {
            let idle_animation = load_animation(resource_manager, definition.idle_animation, model, scene)?;
            let walk_animation = load_animation(resource_manager, definition.walk_animation, model, scene)?;
            let jump_animation = load_animation(resource_manager, definition.jump_animation, model, scene)?;
            let falling_animation = load_animation(resource_manager, definition.falling_animation, model, scene)?;

            let mut machine = Machine::new();

            let jump_node = machine.add_node(machine::PoseNode::make_play_animation(jump_animation));
            let jump_state = machine.add_state(State::new("Jump", jump_node));

            let falling_node = machine.add_node(machine::PoseNode::make_play_animation(falling_animation));
            let falling_state = machine.add_state(State::new("Falling", falling_node));

            let walk_node = machine.add_node(machine::PoseNode::make_play_animation(walk_animation));
            let walk_state = machine.add_state(State::new("Walk", walk_node));

            let idle_node = machine.add_node(machine::PoseNode::make_play_animation(idle_animation));
            let idle_state = machine.add_state(State::new("Idle", idle_node));

            machine.add_transition(machine::Transition::new("Walk->Idle", walk_state, idle_state, 0.5, Self::WALK_TO_IDLE_PARAM))
                .add_transition(machine::Transition::new("Walk->Jump", walk_state, jump_state, 0.5, Self::WALK_TO_JUMP_PARAM))
                .add_transition(machine::Transition::new("Idle->Walk", idle_state, walk_state, 0.5, Self::IDLE_TO_WALK_PARAM))
                .add_transition(machine::Transition::new("Idle->Jump", idle_state, jump_state, 0.5, Self::IDLE_TO_JUMP_PARAM))
                .add_transition(machine::Transition::new("Jump->Falling", jump_state, falling_state, 0.5, Self::JUMP_TO_FALLING_PARAM))
                .add_transition(machine::Transition::new("Falling->Idle", falling_state, idle_state, 0.5, Self::FALLING_TO_IDLE_PARAM));

            machine.set_entry_state(idle_state);

            //  machine.debug(true);

            machine
        };

        let hit_reaction_animation;
        let aim_state;
        let combat_machine = {
            let aim_animation = load_animation(resource_manager, definition.aim_animation, model, scene)?;
            let whip_animation = load_animation(resource_manager, definition.whip_animation, model, scene)?;
            hit_reaction_animation = load_animation(resource_manager, definition.hit_reaction_animation, model, scene)?;

            let SceneInterfaceMut { graph, animations, .. } = scene.interface_mut();

            // These animations must *not* affect legs, because legs animated using locomotion machine
            disable_leg_tracks(animations.get_mut(aim_animation), model, definition.left_leg_name, graph);
            disable_leg_tracks(animations.get_mut(aim_animation), model, definition.right_leg_name, graph);

            disable_leg_tracks(animations.get_mut(whip_animation), model, definition.left_leg_name, graph);
            disable_leg_tracks(animations.get_mut(whip_animation), model, definition.right_leg_name, graph);

            disable_leg_tracks(animations.get_mut(hit_reaction_animation), model, definition.left_leg_name, graph);
            disable_leg_tracks(animations.get_mut(hit_reaction_animation), model, definition.right_leg_name, graph);
            animations.get_mut(hit_reaction_animation).set_loop(false);

            let mut machine = Machine::new();

            let hit_reaction_node = machine.add_node(machine::PoseNode::make_play_animation(hit_reaction_animation));
            let hit_reaction_state = machine.add_state(State::new("HitReaction", hit_reaction_node));

            let aim_node = machine.add_node(machine::PoseNode::make_play_animation(aim_animation));
            aim_state = machine.add_state(State::new("Aim", aim_node));

            let whip_node = machine.add_node(machine::PoseNode::make_play_animation(whip_animation));
            let whip_state = machine.add_state(State::new("Whip", whip_node));

            machine.add_transition(machine::Transition::new("Aim->Whip", aim_state, whip_state, 0.5, Self::AIM_TO_WHIP_PARAM))
                .add_transition(machine::Transition::new("Whip->Aim", whip_state, aim_state, 0.5, Self::WHIP_TO_AIM_PARAM))
                .add_transition(machine::Transition::new("Whip->HitReaction", whip_state, hit_reaction_state, 0.2, Self::WHIP_TO_HIT_REACTION_PARAM))
                .add_transition(machine::Transition::new("Aim->HitReaction", aim_state, hit_reaction_state, 0.2, Self::AIM_TO_HIT_REACTION_PARAM))
                .add_transition(machine::Transition::new("HitReaction->Aim", hit_reaction_state, aim_state, 0.5, Self::HIT_REACTION_TO_AIM_PARAM));

           // machine.debug(true);

            machine
        };

        let dead_state;
        let dying_machine = {
            let dying_animation = load_animation(resource_manager, definition.dying_animation, model, scene)?;
            let dead_animation = load_animation(resource_manager, definition.dead_animation, model, scene)?;

            let mut machine = Machine::new();

            let dying_node = machine.add_node(machine::PoseNode::make_play_animation(dying_animation));
            let dying_state = machine.add_state(State::new("Dying", dying_node));

            let dead_node = machine.add_node(machine::PoseNode::make_play_animation(dead_animation));
            dead_state = machine.add_state(State::new("Dead", dead_node));

            machine.set_entry_state(dying_state);

            machine.add_transition(machine::Transition::new("Dying->Dead", dying_state, dead_state, 1.5, Self::DYING_TO_DEAD));

            machine
        };

        Ok(Self {
            character: Character {
                pivot,
                body,
                weapon_pivot,
                health: definition.health,
                sender: Some(sender),
                ..Default::default()
            },
            definition,
            hit_reaction_animation,
            last_health: definition.health,
            model,
            kind,
            locomotion_machine,
            combat_machine,
            dying_machine,
            dead_state,
            aim_state,
            ..Default::default()
        })
    }

    pub fn can_be_removed(&self) -> bool {
        self.dying_machine.active_state() == self.dead_state
    }

    pub fn can_shoot(&self) -> bool {
        self.combat_machine.active_state() == self.aim_state && self.shoot_interval <= 0.0
    }

    pub fn set_target(&mut self, target: Vec3) {
        self.target = target;
    }

    pub fn set_target_actor(&mut self, actor: Handle<Actor>) {
        self.target_actor.set(actor);
    }
}

impl CleanUp for Bot {
    fn clean_up(&mut self, scene: &mut Scene) {
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
        self.hit_reaction_animation.visit("HitReactionAnimation", visitor)?;
        self.restoration_time.visit("RestorationTime", visitor)?;
        self.dead_state.visit("DeadState", visitor)?;
        self.aim_state.visit("AimState", visitor)?;

        visitor.leave_region()
    }
}