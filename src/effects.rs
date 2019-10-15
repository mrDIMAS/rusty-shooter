use rg3d_core::{
    color_gradient::{GradientPoint, ColorGradient},
    color::Color,
    numeric_range::NumericRange,
    math::vec3::Vec3
};
use std::path::Path;
use rg3d::{
    engine::resource_manager::ResourceManager,
    scene::{
        particle_system::{ParticleSystemBuilder, EmitterKind, EmitterBuilder, SphereEmitter},
        node::{NodeKind, NodeBuilder},
        transform::TransformBuilder,
        graph::Graph
    },
    resource::texture::TextureKind,
};

pub fn create_bullet_impact(graph: &mut Graph, resource_manager: &mut ResourceManager, pos: Vec3) {
    NodeBuilder::new(NodeKind::ParticleSystem(ParticleSystemBuilder::new()
            .with_acceleration(Vec3::make(0.0, 0.0, 0.0))
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
            .build()))
        .with_lifetime(5.0)
        .with_local_transform(TransformBuilder::new()
            .with_local_position(pos)
            .build())
        .build(graph);
}