use rg3d::{
    resource::{
        model::Model,
        texture::TextureKind,
    },
    event::WindowEvent,
    scene::{
        Scene,
        SceneInterfaceMut,
        base::{AsBase, BaseBuilder},
        particle_system::{
            ParticleSystem, Emitter,
            EmitterKind, CustomEmitter, Particle,
            Emit,
        },
        particle_system::{ParticleSystemBuilder, EmitterBuilder},
        node::Node,
    },
    engine::{
        EngineInterfaceMut,
        Engine,
    },
    utils,
};
use std::{
    path::Path
};
use rand::Rng;
use rg3d_core::{
    color::Color,
    color_gradient::{ColorGradient, GradientPoint},
    pool::Handle,
    visitor::{
        Visit,
        VisitResult,
        Visitor,
    },
    math::vec3::*,
};
use crate::{
    actor::{ActorContainer, Actor},
    weapon::{
        Weapon,
        WeaponKind,
        WeaponContainer,
    },
    player::Player,
    GameTime,
    bot::{
        Bot,
        BotKind,
    },
    projectile::ProjectileContainer,
    LevelUpdateContext,
    character::AsCharacter,
    jump_pad::{JumpPadContainer, JumpPad},
};
use crate::item::{ItemContainer, Item, ItemKind};

pub struct Level {
    scene: Handle<Scene>,
    player: Handle<Actor>,
    projectiles: ProjectileContainer,
    actors: ActorContainer,
    weapons: WeaponContainer,
    jump_pads: JumpPadContainer,
    items: ItemContainer,
}

impl Default for Level {
    fn default() -> Self {
        Self {
            projectiles: ProjectileContainer::new(),
            actors: ActorContainer::new(),
            scene: Handle::NONE,
            player: Handle::NONE,
            weapons: WeaponContainer::new(),
            jump_pads: JumpPadContainer::new(),
            items: ItemContainer::new(),
        }
    }
}

pub trait LevelEntity {
    fn update(&mut self, context: &mut LevelUpdateContext);
}

pub trait CleanUp {
    fn clean_up(&mut self, scene: &mut Scene);
}

impl Visit for Level {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.scene.visit("Scene", visitor)?;
        self.player.visit("Player", visitor)?;
        self.actors.visit("Actors", visitor)?;
        self.projectiles.visit("Projectiles", visitor)?;
        self.weapons.visit("Weapons", visitor)?;
        self.jump_pads.visit("JumpPads", visitor)?;

        visitor.leave_region()
    }
}

#[derive(Copy, Clone)]
pub struct CylinderEmitter {
    height: f32,
    radius: f32,
}

impl CylinderEmitter {
    pub fn new() -> Self {
        Self {
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

impl Visit for CylinderEmitter {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.radius.visit("Radius", visitor)?;
        self.height.visit("Height", visitor)?;

        visitor.leave_region()
    }
}

impl Emit for CylinderEmitter {
    fn emit(&self, _emitter: &Emitter, _particle_system: &ParticleSystem, particle: &mut Particle) {
        // Disk point picking extended in 3D - http://mathworld.wolfram.com/DiskPointPicking.html
        let s: f32 = rand::thread_rng().gen_range(0.0, 1.0);
        let theta = rand::thread_rng().gen_range(0.0, 2.0 * std::f32::consts::PI);
        let z = rand::thread_rng().gen_range(0.0, self.height);
        let r = s.sqrt() * self.radius;
        let x = r * theta.cos();
        let y = r * theta.sin();
        particle.position = Vec3::new(x, y, z);
    }
}

impl Level {
    fn load_level(engine: &mut Engine) -> Scene {
        let mut scene = Scene::new();

        let EngineInterfaceMut { resource_manager, .. } = engine.interface_mut();
        let map_model_handle = resource_manager.request_model(Path::new("data/models/dm6.fbx"));
        if map_model_handle.is_some() {
            // Instantiate map
            let map_root_handle = Model::instantiate(map_model_handle.unwrap(), &mut scene).root;
            let SceneInterfaceMut { graph, physics, .. } = scene.interface_mut();
            // Create collision geometry
            let polygon_handle = graph.find_by_name(map_root_handle, "Polygon");
            if polygon_handle.is_some() {
                physics.add_static_geometry(utils::mesh_to_static_geometry(graph.get(polygon_handle).as_mesh()));
            } else {
                println!("Unable to find Polygon node to build collision shape for level!");
            }
        }

        let SceneInterfaceMut { graph, .. } = scene.interface_mut();

        graph.add_node(Node::ParticleSystem(
            ParticleSystemBuilder::new(BaseBuilder::new())
                .with_acceleration(Vec3::new(0.0, -0.1, 0.0))
                .with_color_over_lifetime_gradient({
                    let mut gradient = ColorGradient::new();
                    gradient.add_point(GradientPoint::new(0.00, Color::from_rgba(150, 150, 150, 0)));
                    gradient.add_point(GradientPoint::new(0.05, Color::from_rgba(150, 150, 150, 220)));
                    gradient.add_point(GradientPoint::new(0.85, Color::from_rgba(255, 255, 255, 180)));
                    gradient.add_point(GradientPoint::new(1.00, Color::from_rgba(255, 255, 255, 0)));
                    gradient
                })
                .with_emitters(vec![
                    EmitterBuilder::new(EmitterKind::Custom(Box::new(CylinderEmitter::new()))).build()
                ])
                .with_opt_texture(resource_manager.request_texture(Path::new("data/particles/smoke_04.tga"), TextureKind::R8))
                .build()));

        scene
    }

    pub fn give_weapon(&mut self, engine: &mut Engine, weapon_handle: Handle<Weapon>, actor_handle: Handle<Actor>) {
        let graph = engine.interface_mut().scenes.get_mut(self.scene).interface_mut().graph;
        let weapon = self.weapons.get_mut(weapon_handle);
        let actor = self.actors.get_mut(actor_handle);
        weapon.set_owner(actor_handle);
        actor.character_mut().add_weapon(weapon_handle);
        graph.link_nodes(weapon.get_model(), actor.character().get_weapon_pivot());
    }

    pub fn new(engine: &mut Engine) -> Level {
        let mut scene = Self::load_level(engine);
        let EngineInterfaceMut { scenes, sound_context, resource_manager, .. } = engine.interface_mut();
        let mut actors = ActorContainer::new();
        let mut weapons = WeaponContainer::new();
        let plasma_rifle = weapons.add(Weapon::new(WeaponKind::PlasmaRifle, resource_manager, &mut scene));
        let ak47 = weapons.add(Weapon::new(WeaponKind::Ak47, resource_manager, &mut scene));
        let m4 = weapons.add(Weapon::new(WeaponKind::M4, resource_manager, &mut scene));
        let player = Player::new(sound_context, resource_manager, &mut scene);
        let player = actors.add(Actor::Player(player));
        let scene = scenes.add(scene);
        let mut level = Level {
            weapons,
            actors,
            player,
            scene,
            ..Default::default()
        };
        level.give_weapon(engine, m4, player);
        level.give_weapon(engine, ak47, player);
        level.give_weapon(engine, plasma_rifle, player);
        level.add_bot(engine, Vec3::new(0.0, 0.0, -1.0));
        level.add_bot(engine, Vec3::new(0.0, 0.0, 1.0));
        level.analyze(engine);
        level
    }

    pub fn analyze(&mut self, engine: &mut Engine) {
        let mut items = Vec::new();
        let EngineInterfaceMut { scenes, resource_manager, .. } = engine.interface_mut();
        let scene = scenes.get_mut(self.scene);
        let SceneInterfaceMut { graph, physics, .. } = scene.interface_mut();
        for node in graph.linear_iter() {
            let position = node.base().get_global_position();
            let name = node.base().get_name();
            if name.starts_with("JumpPad") {
                let begin = graph.find_by_name_from_root(format!("{}_Begin", name).as_str());
                let end = graph.find_by_name_from_root(format!("{}_End", name).as_str());
                if begin.is_some() && end.is_some() {
                    let begin = graph.get(begin).base().get_global_position();
                    let end = graph.get(end).base().get_global_position();
                    let (force, len) = (end - begin).normalized_ex();
                    let force = force.unwrap_or(Vec3::UP).scale(len / 20.0);
                    let shape = utils::mesh_to_static_geometry(node.as_mesh());
                    let shape = physics.add_static_geometry(shape);
                    self.jump_pads.add(JumpPad::new(shape, force));
                };
            } else if name.starts_with("Medkit") {
                items.push((ItemKind::Medkit, position));
            } else if name.starts_with("Ammo_Ak47") {
                items.push((ItemKind::Ak47Ammo762, position));
            } else if name.starts_with("Ammo_M4") {
                items.push((ItemKind::M4Ammo556, position));
            } else if name.starts_with("Ammo_Plasma") {
                items.push((ItemKind::Plasma, position));
            }
        }
        for (kind, position) in items {
            self.items.add(Item::new(kind, position, scene, resource_manager));
        }
    }

    pub fn add_bot(&mut self, engine: &mut Engine, position: Vec3) {
        let EngineInterfaceMut { scenes, resource_manager, .. } = engine.interface_mut();
        let scene = scenes.get_mut(self.scene);
        let bot = Actor::Bot(Bot::new(BotKind::Mutant, resource_manager, scene, position).unwrap());
        let bot = self.actors.add(bot);
        // let weapon = self.weapons.add(Weapon::new(WeaponKind::Ak47, resource_manager, scene));
        // self.give_weapon(engine, weapon, bot);
    }

    pub fn destroy(&mut self, engine: &mut Engine) {
        let EngineInterfaceMut { scenes, .. } = engine.interface_mut();
        scenes.remove(self.scene);
    }

    pub fn get_player(&self) -> Handle<Actor> {
        self.player
    }

    pub fn process_input_event(&mut self, event: &WindowEvent) -> bool {
        if let Actor::Player(player) = self.actors.get_mut(self.player) {
            player.process_input_event(event)
        } else {
            false
        }
    }

    pub fn get_actors(&self) -> &ActorContainer {
        &self.actors
    }

    pub fn get_actors_mut(&mut self) -> &mut ActorContainer {
        &mut self.actors
    }

    pub fn get_weapons(&self) -> &WeaponContainer {
        &self.weapons
    }

    pub fn update(&mut self, engine: &mut Engine, time: GameTime) {
        let EngineInterfaceMut { scenes, sound_context, resource_manager, .. } = engine.interface_mut();
        let scene = scenes.get_mut(self.scene);

        let player_position = self.actors.get(self.player).character().get_position(scene.interface().physics);

        for actor in self.actors.iter_mut() {
            if let Actor::Bot(bot) = actor {
                bot.set_target(player_position);
            }
        }

        self.weapons.update(scene);
        self.projectiles.update(scene, resource_manager, &mut self.actors, &self.weapons, time);
        self.items.update(scene,  resource_manager, time);

        let mut context = LevelUpdateContext {
            time,
            scene,
            sound_context,
            resource_manager,
            items: &mut self.items,
            weapons: &mut self.weapons,
            jump_pads: &mut self.jump_pads,
            projectiles: &mut self.projectiles,
        };

        self.actors.update(&mut context);
    }
}