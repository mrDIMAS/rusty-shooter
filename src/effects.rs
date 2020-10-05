use crate::assets;
use rand::Rng;
use rg3d::scene::particle_system::{
    BaseEmitter, BaseEmitterBuilder, Emitter, SphereEmitterBuilder,
};
use rg3d::{
    core::{
        color::Color,
        color_gradient::{ColorGradient, GradientPoint},
        math::vec3::Vec3,
        numeric_range::NumericRange,
        visitor::{Visit, VisitResult, Visitor},
    },
    engine::resource_manager::ResourceManager,
    resource::texture::TextureKind,
    scene::{
        base::BaseBuilder,
        graph::Graph,
        node::Node,
        particle_system::{
            CustomEmitter, CustomEmitterFactory, Emit, Particle, ParticleSystem,
            ParticleSystemBuilder,
        },
        transform::TransformBuilder,
    },
};
use std::ops::{Deref, DerefMut};
use std::path::Path;

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub enum EffectKind {
    BulletImpact,
    ItemAppear,
    Smoke,
    Steam,
}

pub fn create(
    kind: EffectKind,
    graph: &mut Graph,
    resource_manager: &mut ResourceManager,
    pos: Vec3,
) {
    match kind {
        EffectKind::BulletImpact => create_bullet_impact(graph, resource_manager, pos),
        EffectKind::ItemAppear => create_item_appear(graph, resource_manager, pos),
        EffectKind::Smoke => create_smoke(graph, resource_manager, pos),
        EffectKind::Steam => create_steam(graph, resource_manager, pos),
    }
}

#[derive(Clone, Debug)]
pub struct CylinderEmitter {
    base: BaseEmitter,
    height: f32,
    radius: f32,
}

impl CylinderEmitter {
    pub fn new() -> Self {
        Self {
            base: Default::default(),
            height: 1.0,
            radius: 0.5,
        }
    }
}

impl CustomEmitter for CylinderEmitter {
    fn box_clone(&self) -> Box<dyn CustomEmitter> {
        Box::new(self.clone())
    }

    fn get_kind(&self) -> i32 {
        0
    }
}

impl Deref for CylinderEmitter {
    type Target = BaseEmitter;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl DerefMut for CylinderEmitter {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

impl Visit for CylinderEmitter {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.radius.visit("Radius", visitor)?;
        self.height.visit("Height", visitor)?;

        visitor.leave_region()
    }
}

impl Emit for CylinderEmitter {
    fn emit(&self, _particle_system: &ParticleSystem, particle: &mut Particle) {
        // Disk point picking extended in 3D - http://mathworld.wolfram.com/DiskPointPicking.html
        let scale: f32 = rand::thread_rng().gen_range(0.0, 1.0);
        let theta = rand::thread_rng().gen_range(0.0, 2.0 * std::f32::consts::PI);
        let z = rand::thread_rng().gen_range(0.0, self.height);
        let radius = scale.sqrt() * self.radius;
        let x = radius * theta.cos();
        let y = radius * theta.sin();
        particle.position = Vec3::new(x, y, z);
    }
}

pub fn register_custom_emitter_factory() {
    if let Ok(mut factory) = CustomEmitterFactory::get() {
        factory.set_callback(Box::new(|kind| match kind {
            0 => Ok(Box::new(CylinderEmitter::new())),
            _ => Err(String::from("invalid custom emitter kind")),
        }))
    }
}

fn create_steam(graph: &mut Graph, resource_manager: &mut ResourceManager, pos: Vec3) {
    graph.add_node(Node::ParticleSystem(
        ParticleSystemBuilder::new(
            BaseBuilder::new()
                .with_local_transform(TransformBuilder::new().with_local_position(pos).build()),
        )
        .with_acceleration(Vec3::new(0.0, -0.01, 0.0))
        .with_color_over_lifetime_gradient({
            let mut gradient = ColorGradient::new();
            gradient.add_point(GradientPoint::new(0.00, Color::from_rgba(150, 150, 150, 0)));
            gradient.add_point(GradientPoint::new(
                0.05,
                Color::from_rgba(150, 150, 150, 220),
            ));
            gradient.add_point(GradientPoint::new(
                0.85,
                Color::from_rgba(255, 255, 255, 180),
            ));
            gradient.add_point(GradientPoint::new(1.00, Color::from_rgba(255, 255, 255, 0)));
            gradient
        })
        .with_emitters(vec![Emitter::Custom(Box::new(CylinderEmitter {
            base: BaseEmitterBuilder::new().build(),
            height: 0.2,
            radius: 0.2,
        }))])
        .with_opt_texture(resource_manager.request_texture(
            Path::new(assets::textures::particles::SMOKE),
            TextureKind::R8,
        ))
        .build(),
    ));
}

fn create_bullet_impact(graph: &mut Graph, resource_manager: &mut ResourceManager, pos: Vec3) {
    graph.add_node(Node::ParticleSystem(
        ParticleSystemBuilder::new(
            BaseBuilder::new()
                .with_lifetime(1.0)
                .with_local_transform(TransformBuilder::new().with_local_position(pos).build()),
        )
        .with_acceleration(Vec3::new(0.0, -10.0, 0.0))
        .with_color_over_lifetime_gradient({
            let mut gradient = ColorGradient::new();
            gradient.add_point(GradientPoint::new(0.00, Color::from_rgba(255, 255, 0, 0)));
            gradient.add_point(GradientPoint::new(0.05, Color::from_rgba(255, 160, 0, 255)));
            gradient.add_point(GradientPoint::new(0.95, Color::from_rgba(255, 120, 0, 255)));
            gradient.add_point(GradientPoint::new(1.00, Color::from_rgba(255, 60, 0, 0)));
            gradient
        })
        .with_emitters(vec![SphereEmitterBuilder::new(
            BaseEmitterBuilder::new()
                .with_max_particles(200)
                .with_spawn_rate(1000)
                .with_size_modifier_range(NumericRange::new(-0.02, -0.025))
                .with_size_range(NumericRange::new(0.025, 0.05))
                .with_x_velocity_range(NumericRange::new(-0.03, 0.03))
                .with_y_velocity_range(NumericRange::new(0.035, 0.05))
                .with_z_velocity_range(NumericRange::new(-0.03, 0.03))
                .resurrect_particles(false),
        )
        .with_radius(0.01)
        .build()])
        .with_opt_texture(resource_manager.request_texture(
            Path::new(assets::textures::particles::CIRCLE),
            TextureKind::R8,
        ))
        .build(),
    ));
}

fn create_smoke(graph: &mut Graph, resource_manager: &mut ResourceManager, pos: Vec3) {
    graph.add_node(Node::ParticleSystem(
        ParticleSystemBuilder::new(
            BaseBuilder::new()
                .with_lifetime(5.0)
                .with_local_transform(TransformBuilder::new().with_local_position(pos).build()),
        )
        .with_acceleration(Vec3::new(0.0, 0.0, 0.0))
        .with_color_over_lifetime_gradient({
            let mut gradient = ColorGradient::new();
            gradient.add_point(GradientPoint::new(0.00, Color::from_rgba(150, 150, 150, 0)));
            gradient.add_point(GradientPoint::new(
                0.05,
                Color::from_rgba(150, 150, 150, 220),
            ));
            gradient.add_point(GradientPoint::new(
                0.85,
                Color::from_rgba(255, 255, 255, 180),
            ));
            gradient.add_point(GradientPoint::new(1.00, Color::from_rgba(255, 255, 255, 0)));
            gradient
        })
        .with_emitters(vec![SphereEmitterBuilder::new(
            BaseEmitterBuilder::new()
                .with_max_particles(100)
                .with_spawn_rate(50)
                .with_x_velocity_range(NumericRange::new(-0.01, 0.01))
                .with_y_velocity_range(NumericRange::new(0.02, 0.03))
                .with_z_velocity_range(NumericRange::new(-0.01, 0.01)),
        )
        .with_radius(0.01)
        .build()])
        .with_opt_texture(resource_manager.request_texture(
            Path::new(assets::textures::particles::SMOKE),
            TextureKind::R8,
        ))
        .build(),
    ));
}

fn create_item_appear(graph: &mut Graph, resource_manager: &mut ResourceManager, pos: Vec3) {
    graph.add_node(Node::ParticleSystem(
        ParticleSystemBuilder::new(
            BaseBuilder::new()
                .with_lifetime(1.4)
                .with_local_transform(TransformBuilder::new().with_local_position(pos).build()),
        )
        .with_acceleration(Vec3::new(0.0, -6.0, 0.0))
        .with_color_over_lifetime_gradient({
            let mut gradient = ColorGradient::new();
            gradient.add_point(GradientPoint::new(0.00, Color::from_rgba(255, 255, 0, 0)));
            gradient.add_point(GradientPoint::new(0.05, Color::from_rgba(255, 160, 0, 255)));
            gradient.add_point(GradientPoint::new(0.95, Color::from_rgba(255, 120, 0, 255)));
            gradient.add_point(GradientPoint::new(1.00, Color::from_rgba(255, 60, 0, 0)));
            gradient
        })
        .with_emitters(vec![SphereEmitterBuilder::new(
            BaseEmitterBuilder::new()
                .with_max_particles(100)
                .with_spawn_rate(200)
                .with_size_modifier_range(NumericRange::new(-0.012, -0.015))
                .with_size_range(NumericRange::new(0.05, 0.10))
                .with_x_velocity_range(NumericRange::new(-0.02, 0.02))
                .with_y_velocity_range(NumericRange::new(0.035, 0.05))
                .with_z_velocity_range(NumericRange::new(-0.02, 0.02))
                .resurrect_particles(false),
        )
        .with_radius(0.01)
        .build()])
        .with_opt_texture(resource_manager.request_texture(
            Path::new(assets::textures::particles::STAR),
            TextureKind::R8,
        ))
        .build(),
    ));
}
