use std::{
    path::Path,
    cell::Cell,
};
use crate::{
    character::{Character, AsCharacter},
    LevelUpdateContext,
    level::{
        LevelEntity,
        CleanUp,
    },
    actor::Actor,
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
    resource::model::Model,
    engine::resource_manager::ResourceManager,
};

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
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
    definition: &'static BotDefinition,
    pose: AnimationPose,
    locomotion_machine: Machine,
    combat_machine: Machine,
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
            pose: Default::default(),
            locomotion_machine: Default::default(),
            combat_machine: Default::default(),
        }
    }
}

pub struct BotDefinition {
    scale: f32,
    health: f32,
    kind: BotKind,
    walk_speed: f32,
    weapon_scale: f32,
    model: &'static str,
    idle_animation: &'static str,
    walk_animation: &'static str,
    aim_animation: &'static str,
    whip_animation: &'static str,
    jump_animation: &'static str,
    falling_animation: &'static str,
    weapon_hand_name: &'static str,
    left_leg_name: &'static str,
    right_leg_name: &'static str,
}

impl LevelEntity for Bot {
    fn update(&mut self, context: &mut LevelUpdateContext) {
        let SceneInterfaceMut { graph, physics, animations, .. } = context.scene.interface_mut();

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

        self.locomotion_machine.set_parameter(Self::IDLE_TO_WALK_PARAM, machine::Parameter::Rule(distance > threshold));
        self.locomotion_machine.set_parameter(Self::WALK_TO_IDLE_PARAM, machine::Parameter::Rule(distance <= threshold));
        self.locomotion_machine.set_parameter(Self::WALK_TO_JUMP_PARAM, machine::Parameter::Rule(need_jump));
        self.locomotion_machine.set_parameter(Self::IDLE_TO_JUMP_PARAM, machine::Parameter::Rule(need_jump));
        self.locomotion_machine.set_parameter(Self::JUMP_TO_FALLING_PARAM, machine::Parameter::Rule(!has_ground_contact));
        self.locomotion_machine.set_parameter(Self::FALLING_TO_IDLE_PARAM, machine::Parameter::Rule(has_ground_contact));

        self.locomotion_machine.evaluate_pose(animations, context.time.delta).apply(graph);

        // Overwrite upper body with combat machine
        self.combat_machine.set_parameter(Self::WHIP_TO_AIM_PARAM, machine::Parameter::Rule(distance > threshold));
        self.combat_machine.set_parameter(Self::AIM_TO_WHIP_PARAM, machine::Parameter::Rule(distance <= threshold));
        self.combat_machine.evaluate_pose(animations, context.time.delta).apply(graph);

        if distance > threshold && false { // Intentionally disabled.
            if let Some(weapon) = self.character.weapons.get(self.character.current_weapon as usize) {
                let weapon = context.weapons.get_mut(*weapon);
                if let Some(projectile) = weapon.try_shoot(
                    context.scene,
                    context.resource_manager,
                    context.sound_context.clone(),
                    context.time,
                    Vec3::ZERO) {
                    context.projectiles.add(projectile);
                }
            }
        }
    }
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
                    weapon_hand_name: "Mutant:RightHand",
                    left_leg_name: "Mutant:LeftUpLeg",
                    right_leg_name: "Mutant:RightUpLeg",
                    walk_speed: 0.35,
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
                    weapon_hand_name: "RightHand",
                    left_leg_name: "LeftUpLeg",
                    right_leg_name: "RightUpLeg",
                    walk_speed: 0.40,
                    scale: 0.0085,
                    weapon_scale: 2.5,
                    health: 100.0,
                };
                &DEFINITION
            }
            BotKind::Maw => {
                static DEFINITION: BotDefinition = BotDefinition {
                    kind: BotKind::Parasite,
                    model: "data/models/maw.fbx",
                    idle_animation: "data/animations/maw/idle.fbx",
                    walk_animation: "data/animations/maw/walk.fbx",
                    aim_animation: "data/animations/maw/aim.fbx",
                    whip_animation: "data/animations/maw/whip.fbx",
                    jump_animation: "data/animations/maw/jump.fbx",
                    falling_animation: "data/animations/maw/falling.fbx",
                    weapon_hand_name: "RightHand",
                    left_leg_name: "LeftUpLeg",
                    right_leg_name: "RightUpLeg",
                    walk_speed: 0.40,
                    scale: 0.0085,
                    weapon_scale: 2.5,
                    health: 100.0,
                };
                &DEFINITION
            }
        }
    }

    fn disable_leg_tracks(animation: &mut Animation, root: Handle<Node>, leg_name: &str, graph: &Graph) {
        animation.set_tracks_enabled_from(graph.find_by_name(root, leg_name), false, graph)
    }

    pub fn new(kind: BotKind, resource_manager: &mut ResourceManager, scene: &mut Scene, position: Vec3) -> Result<Self, ()> {
        let definition = Self::get_definition(kind);

        let body_height = 1.25;

        let resource = resource_manager.request_model(Path::new(definition.model)).ok_or(())?;
        let model = Model::instantiate_geometry(resource.clone(), scene);
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
            let idle_animation = *Model::retarget_animations(
                resource_manager.request_model(definition.idle_animation.as_ref()).ok_or(())?,
                model, scene).get(0).ok_or(())?;

            let walk_animation = *Model::retarget_animations(
                resource_manager.request_model(definition.walk_animation.as_ref()).ok_or(())?,
                model, scene).get(0).ok_or(())?;

            let jump_animation = *Model::retarget_animations(
                resource_manager.request_model(definition.jump_animation.as_ref()).ok_or(())?,
                model, scene).get(0).ok_or(())?;

            let falling_animation = *Model::retarget_animations(
                resource_manager.request_model(definition.falling_animation.as_ref()).ok_or(())?,
                model, scene).get(0).ok_or(())?;

            let mut machine = Machine::new();

            machine.add_parameter(Self::WALK_TO_IDLE_PARAM, machine::Parameter::Rule(false));
            machine.add_parameter(Self::WALK_TO_JUMP_PARAM, machine::Parameter::Rule(false));
            machine.add_parameter(Self::IDLE_TO_WALK_PARAM, machine::Parameter::Rule(false));
            machine.add_parameter(Self::IDLE_TO_JUMP_PARAM, machine::Parameter::Rule(false));
            machine.add_parameter(Self::JUMP_TO_FALLING_PARAM, machine::Parameter::Rule(false));
            machine.add_parameter(Self::FALLING_TO_IDLE_PARAM, machine::Parameter::Rule(false));

            let jump_node = machine.add_node(machine::PoseNode::make_play_animation(jump_animation));
            let jump_state = machine.add_state(State::new("Jump", jump_node));

            let falling_node = machine.add_node(machine::PoseNode::make_play_animation(falling_animation));
            let falling_state = machine.add_state(State::new("Falling", falling_node));

            let walk_node = machine.add_node(machine::PoseNode::make_play_animation(walk_animation));
            let walk_state = machine.add_state(State::new("Walk", walk_node));

            let idle_node = machine.add_node(machine::PoseNode::make_play_animation(idle_animation));
            let idle_state = machine.add_state(State::new("Idle", idle_node));

            machine.add_transition(machine::Transition::new("Walk->Idle", walk_state, idle_state, 1.0, Self::WALK_TO_IDLE_PARAM));
            machine.add_transition(machine::Transition::new("Walk->Jump", walk_state, jump_state, 0.5, Self::WALK_TO_JUMP_PARAM));
            machine.add_transition(machine::Transition::new("Idle->Walk", idle_state, walk_state, 1.0, Self::IDLE_TO_WALK_PARAM));
            machine.add_transition(machine::Transition::new("Idle->Jump", idle_state, jump_state, 0.5, Self::IDLE_TO_JUMP_PARAM));
            machine.add_transition(machine::Transition::new("Jump->Falling", jump_state, falling_state, 0.5, Self::JUMP_TO_FALLING_PARAM));
            machine.add_transition(machine::Transition::new("Falling->Idle", falling_state, idle_state, 0.5, Self::FALLING_TO_IDLE_PARAM));

            machine.set_entry_state(idle_state);

            machine.debug(true);

            machine
        };

        let combat_machine = {
            let aim_animation = *Model::retarget_animations(
                resource_manager.request_model(definition.aim_animation.as_ref()).ok_or(())?,
                model, scene).get(0).ok_or(())?;

            let whip_animation = *Model::retarget_animations(
                resource_manager.request_model(definition.whip_animation.as_ref()).ok_or(())?,
                model, scene).get(0).ok_or(())?;

            let SceneInterfaceMut { graph, animations, .. } = scene.interface_mut();

            // These animations must *not* affect legs, because legs animated using locomotion machine
            Self::disable_leg_tracks(animations.get_mut(aim_animation), model, definition.left_leg_name, graph);
            Self::disable_leg_tracks(animations.get_mut(aim_animation), model, definition.right_leg_name, graph);

            Self::disable_leg_tracks(animations.get_mut(whip_animation), model, definition.left_leg_name, graph);
            Self::disable_leg_tracks(animations.get_mut(whip_animation), model, definition.right_leg_name, graph);

            let mut machine = Machine::new();

            machine.add_parameter(Self::AIM_TO_WHIP_PARAM, machine::Parameter::Rule(false));
            machine.add_parameter(Self::WHIP_TO_AIM_PARAM, machine::Parameter::Rule(false));

            let aim_node = machine.add_node(machine::PoseNode::PlayAnimation(machine::PlayAnimation::new(aim_animation)));
            let aim_state = machine.add_state(State::new("Aim", aim_node));

            let whip_node = machine.add_node(machine::PoseNode::PlayAnimation(machine::PlayAnimation::new(whip_animation)));
            let whip_state = machine.add_state(State::new("Whip", whip_node));

            machine.add_transition(
                machine::Transition::new("Aim->Whip", aim_state, whip_state, 1.0, Self::AIM_TO_WHIP_PARAM));
            machine.add_transition(
                machine::Transition::new("Whip->Aim", whip_state, aim_state, 1.0, Self::WHIP_TO_AIM_PARAM));

            machine
        };

        Ok(Self {
            character: Character {
                pivot,
                body,
                weapon_pivot,
                health: definition.health,
                ..Default::default()
            },
            model,
            kind,
            locomotion_machine,
            combat_machine,
            ..Default::default()
        })
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

        visitor.leave_region()
    }
}