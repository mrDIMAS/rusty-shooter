use rg3d_core::{
    color_gradient::{GradientPoint, ColorGradient},
    color::Color,
    numeric_range::NumericRange,
    math::vec3::Vec3,
};
use std::path::Path;
use rg3d::{
    engine::resource_manager::ResourceManager,
    scene::{
        particle_system::{ParticleSystemBuilder, EmitterKind, EmitterBuilder, SphereEmitter},
        node::Node,
        transform::TransformBuilder,
        graph::Graph,
        base::BaseBuilder
    },
    resource::texture::TextureKind,
};

pub fn create_bullet_impact(graph: &mut Graph, resource_manager: &mut ResourceManager, pos: Vec3) {
    graph.add_node(Node::ParticleSystem(ParticleSystemBuilder::new(BaseBuilder::new()
        .with_lifetime(5.0)
        .with_local_transform(TransformBuilder::new()
            .with_local_position(pos)
            .build()))
        .with_acceleration(Vec3::new(0.0, 0.0, 0.0))
        .with_color_over_lifetime_gradient({
            let mut gradient = ColorGradient::new();
            gradient.add_point(GradientPoint::new(0.00, Color::from_rgba(150, 150, 150, 0)));
            gradient.add_point(GradientPoint::new(0.05, Color::from_rgba(150, 150, 150, 220)));
            gradient.add_point(GradientPoint::new(0.85, Color::from_rgba(255, 255, 255, 180)));
            gradient.add_point(GradientPoint::new(1.00, Color::from_rgba(255, 255, 255, 0)));
            gradient
        })
        .with_emitters(vec![
            EmitterBuilder::new(EmitterKind::Sphere(SphereEmitter::new(0.01)))
                .with_max_particles(100)
                .with_spawn_rate(50)
                .with_x_velocity_range(NumericRange::new(-0.01, 0.01))
                .with_y_velocity_range(NumericRange::new(0.02, 0.03))
                .with_z_velocity_range(NumericRange::new(-0.01, 0.01))
                .build()
        ])
        .with_opt_texture(resource_manager.request_texture(Path::new("data/particles/smoke_04.tga"), TextureKind::R8))
        .build()));
}

pub fn create_item_appear(graph: &mut Graph, resource_manager: &mut ResourceManager, pos: Vec3) {
    graph.add_node(Node::ParticleSystem(ParticleSystemBuilder::new(BaseBuilder::new()
        .with_lifetime(1.4)
        .with_local_transform(TransformBuilder::new()
            .with_local_position(pos)
            .build()))
        .with_acceleration(Vec3::new(0.0, -6.0, 0.0))
        .with_color_over_lifetime_gradient({
            let mut gradient = ColorGradient::new();
            gradient.add_point(GradientPoint::new(0.00, Color::from_rgba(255, 255, 0, 0)));
            gradient.add_point(GradientPoint::new(0.05, Color::from_rgba(255, 160, 0, 255)));
            gradient.add_point(GradientPoint::new(0.95, Color::from_rgba(255, 120, 0, 255)));
            gradient.add_point(GradientPoint::new(1.00, Color::from_rgba(255, 60, 0, 0)));
            gradient
        })
        .with_emitters(vec![
            EmitterBuilder::new(EmitterKind::Sphere(SphereEmitter::new(0.01)))
                .with_max_particles(100)
                .with_spawn_rate(200)
                .with_size_modifier_range(NumericRange::new(-0.012, -0.015))
                .with_size_range(NumericRange::new(0.05, 0.10))
                .with_x_velocity_range(NumericRange::new(-0.02, 0.02))
                .with_y_velocity_range(NumericRange::new(0.035, 0.05))
                .with_z_velocity_range(NumericRange::new(-0.02, 0.02))
                .resurrect_particles(false)
                .build()
        ])
        .with_opt_texture(resource_manager.request_texture(Path::new("data/particles/star_09.png"), TextureKind::R8))
        .build()));
}
