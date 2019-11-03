use rg3d_core::{
    pool::Handle,
    visitor::{Visit, VisitResult, Visitor},
    math::{vec3::Vec3, quat::Quat},
};
use std::{
    path::Path,
};
use rg3d_physics::{
    rigid_body::RigidBody,
    convex_shape::{ConvexShape, CapsuleShape, Axis},
};
use crate::{
    character::{Character, AsCharacter},
    LevelUpdateContext,
    level::{
        LevelEntity,
        CleanUp,
    },
};
use rg3d::{
    scene::{
        node::Node,
        animation::Animation,
        Scene,
        SceneInterfaceMut,
        base::{
            AsBase,
            BaseBuilder
        },
        transform::TransformBuilder
    },
    resource::model::Model,
    engine::resource_manager::ResourceManager,
};

pub enum BotKind {
    Mutant,
    Ripper,
}

impl BotKind {
    pub fn new(id: i32) -> Result<Self, String> {
        match id {
            0 => Ok(BotKind::Mutant),
            1 => Ok(BotKind::Ripper),
            _ => Err(format!("Invalid bot kind {}", id))
        }
    }

    pub fn id(&self) -> i32 {
        match self {
            BotKind::Mutant => 0,
            BotKind::Ripper => 1,
        }
    }
}

pub struct Bot {
    character: Character,
    model: Handle<Node>,
    kind: BotKind,
    idle_animation: Handle<Animation>,
    walk_animation: Handle<Animation>,
    target: Vec3,
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
            model: Handle::NONE,
            idle_animation: Handle::NONE,
            walk_animation: Handle::NONE,
            target: Vec3::ZERO,
        }
    }
}

impl LevelEntity for Bot {
    fn update(&mut self, context: &mut LevelUpdateContext) {
        let SceneInterfaceMut { graph, physics, animations, .. } = context.scene.interface_mut();

        let threshold = 2.0;
        let body = physics.borrow_body_mut(self.character.body);
        let dir = self.target - body.get_position();
        let distance = dir.len();

        if let Some(dir) = dir.normalized() {
            if distance > threshold {
                body.move_by(dir.scale(0.35 * context.time.delta));
            }

            let pivot = graph.get_mut(self.character.pivot);
            let transform = pivot.base_mut().get_local_transform_mut();
            let angle = dir.x.atan2(dir.z);
            transform.set_rotation(Quat::from_axis_angle(Vec3::UP, angle))
        }

        let fade_speed = 1.5;

        if distance > threshold {
            let walk_animation = animations.get_mut(self.walk_animation);
            walk_animation.fade_in(fade_speed);
            walk_animation.set_enabled(true);

            let idle_animation = animations.get_mut(self.idle_animation);
            idle_animation.fade_out(fade_speed);
            idle_animation.set_enabled(true);
        } else {
            let walk_animation = animations.get_mut(self.walk_animation);
            walk_animation.fade_out(fade_speed);
            walk_animation.set_enabled(true);

            let idle_animation = animations.get_mut(self.idle_animation);
            idle_animation.fade_in(fade_speed);
            idle_animation.set_enabled(true);
        }
    }
}

impl Bot {
    pub fn new(kind: BotKind, resource_manager: &mut ResourceManager, scene: &mut Scene, position: Vec3) -> Result<Self, ()> {
        let path = match kind {
            BotKind::Mutant => Path::new("data/models/mutant.FBX"),
            BotKind::Ripper => Path::new("data/models/ripper.fbx"),
        };

        let body_height = 1.25;

        let scale = match kind {
            BotKind::Mutant => 0.025,
            _ => 1.0,
        };

        let resource = resource_manager.request_model(path).ok_or(())?;
        let model = Model::instantiate_geometry(resource.clone(), scene);
        let (pivot, body) = {
            let SceneInterfaceMut { graph, physics, node_rigid_body_map, .. } = scene.interface_mut();
            let pivot = graph.add_node(Node::Base(Default::default()));
            graph.link_nodes(model, pivot);
            let transform = graph.get_mut(model).base_mut().get_local_transform_mut();
            transform.set_position(Vec3::new(0.0, -body_height * 0.5, 0.0));
            transform.set_scale(Vec3::new(scale, scale, scale));

            let capsule_shape = CapsuleShape::new(0.35, body_height, Axis::Y);
            let mut capsule_body = RigidBody::new(ConvexShape::Capsule(capsule_shape));
            capsule_body.set_position(position);
            let body = physics.add_body(capsule_body);
            node_rigid_body_map.insert(pivot, body);

            (pivot, body)
        };

        let idle_animation = *Model::retarget_animations(
            resource_manager.request_model(
                Path::new("data/animations/idle.fbx")).ok_or(())?,
            model, scene,
        ).get(0).ok_or(())?;

        let walk_animation = *Model::retarget_animations(
            resource_manager.request_model(
                Path::new("data/animations/walk.fbx")).ok_or(())?,
            model, scene,
        ).get(0).ok_or(())?;

        let hand = scene.interface().graph.find_by_name(model, "Mutant:LeftHand");
        let inv_scale = 5.0 * ( 1.0 / scale);
        let weapon_pivot = Node::Base(BaseBuilder::new()
            .with_local_transform(TransformBuilder::new()
                .with_local_scale(Vec3::new(inv_scale, inv_scale, inv_scale))
                .build())
            .build());
        let graph = scene.interface_mut().graph;
        let weapon_pivot = graph.add_node(weapon_pivot);
        graph.link_nodes(weapon_pivot, hand);

        Ok(Self {
            character: Character {
                pivot,
                body,
                weapon_pivot,
                ..Default::default()
            },
            model,
            kind,
            idle_animation,
            walk_animation,
            target: Vec3::ZERO,
        })
    }

    pub fn set_target(&mut self, target: Vec3) {
        self.target = target;
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

        self.character.visit("Character", visitor)?;
        self.model.visit("Model", visitor)?;
        self.idle_animation.visit("IdleAnimation", visitor)?;
        self.walk_animation.visit("WalkAnimation", visitor)?;

        visitor.leave_region()
    }
}