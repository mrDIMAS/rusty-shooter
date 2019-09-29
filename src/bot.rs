use rg3d_core::{
    pool::Handle,
    visitor::{
        Visit,
        VisitResult,
        Visitor,
    },
    math::vec3::Vec3,
};
use rg3d::{
    engine::Engine,
    scene::{
        node::Node,
        Scene,
    },
    resource::model::Model,
};
use std::path::Path;
use rg3d_physics::{
    Body,
    shape::{ConvexShape, CapsuleShape, Axis},
};
use rg3d::scene::animation::Animation;
use crate::GameTime;
use rg3d_core::math::quat::Quat;
use rg3d::scene::node::NodeKind;

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
    kind: BotKind,
    model: Handle<Node>,
    body: Handle<Body>,
    idle_animation: Handle<Animation>,
    walk_animation: Handle<Animation>,
}

impl Default for Bot {
    fn default() -> Self {
        Self {
            pivot: Handle::none(),
            kind: BotKind::Mutant,
            model: Handle::none(),
            body: Handle::none(),
            idle_animation: Handle::none(),
            walk_animation: Handle::none(),
        }
    }
}

impl Bot {
    pub fn new(kind: BotKind, engine: &mut Engine, scene: &mut Scene) -> Result<Self, ()> {
        let path = match kind {
            BotKind::Mutant => Path::new("data/models/mutant.fbx"),
            BotKind::Ripper => Path::new("data/models/ripper.fbx"),
        };

        let body_height = 1.25;
        let resource = engine.get_state_mut().request_model(path).ok_or(())?;
        let pivot = scene.add_node(Node::new(NodeKind::Base));
        let model = Model::instantiate_geometry(resource.clone(), scene);
        scene.link_nodes(model, pivot);
        if let Some(model) = scene.get_node_mut(model) {
            model.get_local_transform_mut().set_position(Vec3::make(0.0, -body_height * 0.5, 0.0));
        }

        match kind {
            BotKind::Mutant => {
                if let Some(model) = scene.get_node_mut(model) {
                    model.get_local_transform_mut().set_scale(Vec3::make(0.025, 0.025, 0.025));
                }
            }
            _ => {}
        }

        let idle_animation = *Model::retarget_animations(
            engine.get_state_mut().request_model(
                Path::new("data/animations/idle.fbx")).ok_or(())?,
            model, scene,
        ).get(0).ok_or(())?;

        let walk_animation = *Model::retarget_animations(
            engine.get_state_mut().request_model(
                Path::new("data/animations/walk.fbx")).ok_or(())?,
            model, scene,
        ).get(0).ok_or(())?;

        let capsule_shape = CapsuleShape::new(0.25, body_height, Axis::Y);
        let capsule_body = Body::new(ConvexShape::Capsule(capsule_shape));
        let body = scene.get_physics_mut().add_body(capsule_body);

        if let Some(pivot) = scene.get_node_mut(pivot) {
            pivot.set_body(body);
        }

        Ok(Self {
            pivot,
            model,
            kind,
            body,
            idle_animation,
            walk_animation,
        })
    }

    pub fn update(&mut self, scene: &mut Scene, player_position: Vec3, time: &GameTime) {
        let threshold = 2.0;

        let mut distance = 0.0;

        if let Some(body) = scene.get_physics_mut().borrow_body_mut(self.body) {
            let dir = player_position - body.get_position();
            distance = dir.len();

            if let Some(dir) = dir.normalized() {
                if distance > threshold {
                    body.move_by(dir.scale(0.35 * time.delta));
                }

                if let Some(pivot) = scene.get_node_mut(self.pivot) {
                    let transform = pivot.get_local_transform_mut();
                    let angle = dir.x.atan2(dir.z);
                    transform.set_rotation(Quat::from_axis_angle(Vec3::up(), angle))
                }
            }
        }

        let fade_speed = 1.5;

        if distance > threshold {
            if let Some(walk_animation) = scene.get_animation_mut(self.walk_animation) {
                walk_animation.fade_in(fade_speed);
                walk_animation.set_enabled(true);
            }
            if let Some(idle_animation) = scene.get_animation_mut(self.idle_animation) {
                idle_animation.fade_out(fade_speed);
                idle_animation.set_enabled(true);
            }
        } else {
            if let Some(walk_animation) = scene.get_animation_mut(self.walk_animation) {
                walk_animation.fade_out(fade_speed);
                walk_animation.set_enabled(true);
            }
            if let Some(idle_animation) = scene.get_animation_mut(self.idle_animation) {
                idle_animation.fade_in(fade_speed);
                idle_animation.set_enabled(true);
            }
        }
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

        visitor.leave_region()
    }
}