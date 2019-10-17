use rg3d_core::{
    pool::Handle,
    visitor::{Visit, VisitResult, Visitor},
    math::{vec3::Vec3, quat::Quat},
};
use std::{
    path::Path,
    sync::{Arc, Mutex}
};
use rg3d_physics::{
    rigid_body::RigidBody,
    convex_shape::{ConvexShape, CapsuleShape, Axis},
};
use rg3d::{
    scene::{
        node::{NodeTrait, Node},
        animation::Animation,
        Scene,
        SceneInterfaceMut,
    },
    resource::model::Model,
    engine::resource_manager::ResourceManager,
};
use crate::{
    actor::ActorTrait,
    GameTime,
    projectile::ProjectileContainer
};
use rg3d_sound::context::Context;

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
    pivot: Handle<Node>,
    model: Handle<Node>,
    body: Handle<RigidBody>,
    health: f32,
    kind: BotKind,
    idle_animation: Handle<Animation>,
    walk_animation: Handle<Animation>,
    target: Vec3,
}

impl Default for Bot {
    fn default() -> Self {
        Self {
            pivot: Handle::NONE,
            kind: BotKind::Mutant,
            model: Handle::NONE,
            body: Handle::NONE,
            idle_animation: Handle::NONE,
            walk_animation: Handle::NONE,
            target: Vec3::ZERO,
            health: 0.0,
        }
    }
}

impl ActorTrait for Bot {
    fn get_body(&self) -> Handle<RigidBody> {
        self.body
    }

    fn get_health(&self) -> f32 {
        self.health
    }

    fn set_health(&mut self, health: f32) {
        self.health = health;
    }

    fn remove_self(&self, scene: &mut Scene) {}

    fn update(&mut self,
              sound_context: Arc<Mutex<Context>>,
              resource_manager: &mut ResourceManager,
              scene: &mut Scene,
              time: &GameTime,
              projectiles: &mut ProjectileContainer,
    ) {
        let SceneInterfaceMut { graph, physics, animations, .. } = scene.interface_mut();

        let threshold = 2.0;
        let body = physics.borrow_body_mut(self.body);
        let dir = self.target - body.get_position();
        let distance = dir.len();

        if let Some(dir) = dir.normalized() {
            if distance > threshold {
                body.move_by(dir.scale(0.35 * time.delta));
            }

            let pivot = graph.get_mut(self.pivot);
            let transform = pivot.get_local_transform_mut();
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
            BotKind::Mutant => Path::new("data/models/mutant.fbx"),
            BotKind::Ripper => Path::new("data/models/ripper.fbx"),
        };

        let body_height = 1.25;

        let resource = resource_manager.request_model(path).ok_or(())?;
        let model = Model::instantiate_geometry(resource.clone(), scene);
        let (pivot, body) = {
            let SceneInterfaceMut { graph, physics, node_rigid_body_map, .. } = scene.interface_mut();
            let pivot = graph.add_node(Node::Pivot(Default::default()));
            graph.link_nodes(model, pivot);
            graph.get_mut(model).get_local_transform_mut().set_position(Vec3::new(0.0, -body_height * 0.5, 0.0));

            match kind {
                BotKind::Mutant => {
                    graph.get_mut(model).get_local_transform_mut().set_scale(Vec3::new(0.025, 0.025, 0.025));
                }
                _ => {}
            }

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

        Ok(Self {
            pivot,
            model,
            kind,
            body,
            idle_animation,
            walk_animation,
            health: 100.0,
            target: Vec3::ZERO,
        })
    }

    pub fn set_target(&mut self, target: Vec3) {
        self.target = target;
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

        self.pivot.visit("Pivot", visitor)?;
        self.model.visit("Model", visitor)?;
        self.body.visit("Body", visitor)?;
        self.idle_animation.visit("IdleAnimation", visitor)?;
        self.walk_animation.visit("WalkAnimation", visitor)?;
        self.health.visit("Health", visitor)?;

        visitor.leave_region()
    }
}