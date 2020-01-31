use std::{
    path::Path,
    cell::RefCell,
    rc::Rc,
    sync::{
        Mutex,
        Arc,
        mpsc::Sender,
    },
    path::PathBuf,
};
use rand::Rng;
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
    projectile::{
        ProjectileContainer,
        ProjectileKind,
        Projectile,
    },
    character::AsCharacter,
    jump_pad::{JumpPadContainer, JumpPad},
    item::{ItemContainer, Item, ItemKind},
    ControlScheme,
    effects::{
        self,
        EffectKind,
    },
};
use rg3d::{
    engine::Engine,
    resource::{
        texture::TextureKind,
    },
    event::Event,
    scene::{
        Scene,
        base::{AsBase, BaseBuilder},
        particle_system::{
            ParticleSystem, Emitter,
            EmitterKind, CustomEmitter, Particle,
            Emit,
        },
        particle_system::{ParticleSystemBuilder, EmitterBuilder},
        node::Node,
        transform::TransformBuilder,
    },
    utils::{
        self,
        navmesh::Navmesh,
    },
    core::{
        color::Color,
        color_gradient::{ColorGradient, GradientPoint},
        pool::Handle,
        visitor::{
            Visit,
            VisitResult,
            Visitor,
        },
        math::{
            vec3::*,
            ray::Ray,
            aabb::AxisAlignedBoundingBox,
        },
    },
    physics::RayCastOptions,
    sound::{
        context::Context,
        source::{
            generic::GenericSourceBuilder,
            spatial::SpatialSourceBuilder,
            Status,
        },
    },
};
use rg3d::renderer::debug_renderer;
use crate::rg3d::core::math::PositionProvider;

pub struct Level {
    pub scene: Handle<Scene>,
    player: Handle<Actor>,
    projectiles: ProjectileContainer,
    pub actors: ActorContainer,
    weapons: WeaponContainer,
    jump_pads: JumpPadContainer,
    items: ItemContainer,
    spawn_points: Vec<SpawnPoint>,
    events_sender: Option<Sender<GameEvent>>,
    pub navmesh: Option<Navmesh>,
    pub control_scheme: Option<Rc<RefCell<ControlScheme>>>,
    death_zones: Vec<DeathZone>,
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
            spawn_points: Default::default(),
            events_sender: None,
            navmesh: Default::default(),
            control_scheme: None,
            death_zones: Default::default(),
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
        self.spawn_points.visit("SpawnPoints", visitor)?;
        self.death_zones.visit("DeathZones", visitor)?;

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

pub struct DeathZone {
    bounds: AxisAlignedBoundingBox
}

impl Visit for DeathZone {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.bounds.visit("Bounds", visitor)?;

        visitor.leave_region()
    }
}

impl Default for DeathZone {
    fn default() -> Self {
        Self {
            bounds: Default::default()
        }
    }
}

pub struct LevelUpdateContext<'a> {
    pub time: GameTime,
    pub scene: &'a mut Scene,
    pub sound_context: Arc<Mutex<Context>>,
    pub items: &'a ItemContainer,
    pub jump_pads: &'a JumpPadContainer,
    pub navmesh: Option<&'a mut Navmesh>,
}

impl Level {
    fn load_level(engine: &mut Engine) -> (Scene, Option<Navmesh>) {
        let mut scene = Scene::new();
        let mut navmesh = None;

        let map_model = engine.resource_manager.request_model(Path::new("data/models/dm6.fbx"));
        if map_model.is_some() {
            // Instantiate map
            let map_root_handle = map_model.unwrap().lock().unwrap().instantiate(&mut scene).root;
            // Create collision geometry
            let polygon_handle = scene.graph.find_by_name(map_root_handle, "Polygon");
            if polygon_handle.is_some() {
                scene.physics.add_static_geometry(utils::mesh_to_static_geometry(scene.graph.get(polygon_handle).as_mesh()));
            } else {
                println!("Unable to find Polygon node to build collision shape for level!");
            }

            let navmesh_handle = scene.graph.find_by_name(map_root_handle, "Navmesh");
            if navmesh_handle.is_some() {
                let navmesh_node = scene.graph.get_mut(navmesh_handle);
                navmesh_node.base_mut().set_visibility(false);
                navmesh = Some(utils::mesh_to_navmesh(navmesh_node.as_mesh()));
            } else {
                println!("Unable to find Navmesh node to build navmesh!")
            }
        }

        scene.graph.add_node(Node::ParticleSystem(
            ParticleSystemBuilder::new(BaseBuilder::new()
                .with_local_transform(TransformBuilder::new()
                    .with_local_position(Vec3::new(0.0, 1.0, 0.0))
                    .build()))
                .with_acceleration(Vec3::new(0.0, -0.01, 0.0))
                .with_color_over_lifetime_gradient({
                    let mut gradient = ColorGradient::new();
                    gradient.add_point(GradientPoint::new(0.00, Color::from_rgba(150, 150, 150, 0)));
                    gradient.add_point(GradientPoint::new(0.05, Color::from_rgba(150, 150, 150, 220)));
                    gradient.add_point(GradientPoint::new(0.85, Color::from_rgba(255, 255, 255, 180)));
                    gradient.add_point(GradientPoint::new(1.00, Color::from_rgba(255, 255, 255, 0)));
                    gradient
                })
                .with_emitters(vec![
                    EmitterBuilder::new(EmitterKind::Custom(Box::new(CylinderEmitter { height: 0.2, radius: 0.2 }))).build()
                ])
                .with_opt_texture(engine.resource_manager.request_texture(Path::new("data/particles/smoke_04.tga"), TextureKind::R8))
                .build()));

        (scene, navmesh)
    }

    pub fn new(engine: &mut Engine, control_scheme: Rc<RefCell<ControlScheme>>, sender: Sender<GameEvent>) -> Level {
        let (scene, navmesh) = Self::load_level(engine);

        let mut level = Level {
            navmesh,
            scene: engine.scenes.add(scene),
            events_sender: Some(sender.clone()),
            control_scheme: Some(control_scheme),
            ..Default::default()
        };

        level.spawn_player(engine);

        level.add_bot(engine, BotKind::Maw, Vec3::new(0.0, 0.0, -1.0));
        level.add_bot(engine, BotKind::Mutant, Vec3::new(0.0, 0.0, 1.0));
        level.add_bot(engine, BotKind::Parasite, Vec3::new(1.0, 0.0, 0.0));

        level.analyze(engine);

        level
    }

    pub fn analyze(&mut self, engine: &mut Engine) {
        let mut items = Vec::new();
        let mut spawn_points = Vec::new();
        let mut death_zones = Vec::new();
        let scene = engine.scenes.get_mut(self.scene);
        for (handle, node) in scene.graph.pair_iter() {
            let position = node.base().get_global_position();
            let name = node.base().get_name();
            if name.starts_with("JumpPad") {
                let begin = scene.graph.find_by_name_from_root(format!("{}_Begin", name).as_str());
                let end = scene.graph.find_by_name_from_root(format!("{}_End", name).as_str());
                if begin.is_some() && end.is_some() {
                    let begin = scene.graph.get(begin).base().get_global_position();
                    let end = scene.graph.get(end).base().get_global_position();
                    let (force, len) = (end - begin).normalized_ex();
                    let force = force.unwrap_or(Vec3::UP).scale(len / 20.0);
                    let shape = utils::mesh_to_static_geometry(node.as_mesh());
                    let shape = scene.physics.add_static_geometry(shape);
                    self.jump_pads.add(JumpPad::new(shape, force));
                };
            } else if name.starts_with("Medkit") {
                items.push((ItemKind::Medkit, position));
            } else if name.starts_with("Ammo_Ak47") {
                items.push((ItemKind::Ak47Ammo, position));
            } else if name.starts_with("Ammo_M4") {
                items.push((ItemKind::M4Ammo, position));
            } else if name.starts_with("Ammo_Plasma") {
                items.push((ItemKind::Plasma, position));
            } else if name.starts_with("SpawnPoint") {
                spawn_points.push(node.base().get_global_position())
            } else if name.starts_with("DeathZone") {
                if let Node::Mesh(mesh) = node {
                    death_zones.push(handle);
                }
            }
        }
        for (kind, position) in items {
            self.items.add(Item::new(kind, position, scene, &mut engine.resource_manager, self.events_sender.as_ref().unwrap().clone()));
        }
        for handle in death_zones {
            let node = scene.graph.get_mut(handle);
            node.base_mut().set_visibility(false);
            self.death_zones.push(DeathZone { bounds: node.as_mesh().calculate_world_bounding_box() });
        }
        self.spawn_points = spawn_points.into_iter().map(|p| SpawnPoint { position: p }).collect();
    }

    pub fn destroy(&mut self, engine: &mut Engine) {
        engine.scenes.remove(self.scene);
    }

    pub fn get_player(&self) -> Handle<Actor> {
        self.player
    }

    pub fn process_input_event(&mut self, event: &Event<()>) -> bool {
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

    fn pick(&self, engine: &mut Engine, from: Vec3, to: Vec3) -> Vec3 {
        let scene = engine.scenes.get_mut(self.scene);
        if let Some(ray) = Ray::from_two_points(&from, &to) {
            let mut intersections = Vec::new();
            let options = RayCastOptions { ignore_bodies: true, ..Default::default() };
            scene.physics.ray_cast(&ray, options, &mut intersections);
            if let Some(pt) = intersections.first() {
                pt.position
            } else {
                from
            }
        } else {
            from
        }
    }

    fn remove_weapon(&mut self, engine: &mut Engine, weapon: Handle<Weapon>) {
        let scene = engine.scenes.get_mut(self.scene);
        for projectile in self.projectiles.iter_mut() {
            if projectile.owner == weapon {
                // Reset owner because handle to weapon will be invalid after weapon freed.
                projectile.owner = Handle::NONE;
            }
        }
        self.weapons.get_mut(weapon).clean_up(scene);
        self.weapons.free(weapon);
    }

    fn give_new_weapon(&mut self, engine: &mut Engine, actor: Handle<Actor>, kind: WeaponKind) {
        if self.actors.contains(actor) {
            let scene = engine.scenes.get_mut(self.scene);
            let mut weapon = Weapon::new(kind, &mut engine.resource_manager, scene, self.events_sender.as_ref().unwrap().clone());
            weapon.set_owner(actor);
            let weapon_model = weapon.get_model();
            let actor = self.actors.get_mut(actor);
            let weapon_handle = self.weapons.add(weapon);
            actor.character_mut().add_weapon(weapon_handle);
            scene.graph.link_nodes(weapon_model, actor.character().get_weapon_pivot());

            self.events_sender
                .as_ref()
                .unwrap()
                .send(GameEvent::AddNotification {
                    text: format!("Actor picked up weapon {:?}", kind)
                }).unwrap();
        }
    }

    fn add_bot(&mut self, engine: &mut Engine, kind: BotKind, position: Vec3) {
        let scene = engine.scenes.get_mut(self.scene);
        let mut bot = Bot::new(kind, &mut engine.resource_manager, scene, position, self.events_sender.as_ref().unwrap().clone()).unwrap();
        bot.set_target_actor(self.player); // TODO: This is temporary until there is no automatic target selection.
        let bot = self.actors.add(Actor::Bot(bot));
        self.events_sender.as_ref().unwrap().send(GameEvent::GiveNewWeapon { actor: bot, kind: WeaponKind::Ak47 }).unwrap();
        println!("Bot {:?} was added!", kind);
    }

    fn remove_actor(&mut self, engine: &mut Engine, actor: Handle<Actor>) {
        if self.actors.contains(actor) {
            let scene = engine.scenes.get_mut(self.scene);
            let character = self.actors.get(actor).character();

            // Make sure to remove weapons and drop appropriate items (items will be temporary).
            let drop_position = character.get_position(&scene.physics);
            let weapons = character.get_weapons()
                .iter()
                .map(|h| *h)
                .collect::<Vec<Handle<Weapon>>>();
            for weapon in weapons {
                let item_kind = match self.weapons.get(weapon).get_kind() {
                    WeaponKind::M4 => ItemKind::M4,
                    WeaponKind::Ak47 => ItemKind::Ak47,
                    WeaponKind::PlasmaRifle => ItemKind::PlasmaGun,
                };
                self.spawn_item(engine, item_kind, drop_position, true, Some(20.0));
                self.remove_weapon(engine, weapon);
            }

            let scene = engine.scenes.get_mut(self.scene);
            self.actors
                .get_mut(actor)
                .character_mut()
                .clean_up(scene);
            self.actors.free(actor);

            if self.player == actor {
                self.player = Handle::NONE;
            }
        }
    }

    fn spawn_player(&mut self, engine: &mut Engine) {
        let index = self.find_suitable_spawn_point(engine);
        let spawn_position = self.spawn_points
            .get(index)
            .map_or(Vec3::ZERO, |pt| pt.position);
        let scene = engine.scenes.get_mut(self.scene);
        let resource_manager = &mut engine.resource_manager;
        let mut player = Player::new(engine.sound_context.clone(), resource_manager, scene, self.events_sender.as_ref().unwrap().clone());
        if let Some(control_scheme) = self.control_scheme.as_ref() {
            player.set_control_scheme(control_scheme.clone());
        }
        self.player = self.actors.add(Actor::Player(player));
        self.actors
            .get_mut(self.player)
            .character_mut()
            .set_position(&mut scene.physics, spawn_position);

        self.give_new_weapon(engine, self.player, WeaponKind::M4);
        self.give_new_weapon(engine, self.player, WeaponKind::Ak47);
        self.give_new_weapon(engine, self.player, WeaponKind::PlasmaRifle);
    }

    fn give_item(&mut self, engine: &mut Engine, actor: Handle<Actor>, kind: ItemKind) {
        if self.actors.contains(actor) {
            let character = self.actors.get_mut(actor).character_mut();
            match kind {
                ItemKind::Medkit => character.heal(20.0),
                ItemKind::Ak47 | ItemKind::PlasmaGun | ItemKind::M4 => {
                    let weapon_kind = match kind {
                        ItemKind::Ak47 => WeaponKind::Ak47,
                        ItemKind::PlasmaGun => WeaponKind::PlasmaRifle,
                        ItemKind::M4 => WeaponKind::M4,
                        _ => unreachable!()
                    };

                    let mut found = false;
                    for weapon_handle in character.get_weapons() {
                        let weapon = self.weapons.get_mut(*weapon_handle);
                        // If actor already has weapon of given kind, then just add ammo to it.
                        if weapon.get_kind() == weapon_kind {
                            found = true;
                            weapon.add_ammo(30);
                            break;
                        }
                    }
                    // Finally if actor does not have such weapon, give new one to him.
                    if !found {
                        self.give_new_weapon(engine, actor, weapon_kind);
                    }
                }
                ItemKind::Plasma | ItemKind::Ak47Ammo | ItemKind::M4Ammo => {
                    for weapon in character.get_weapons() {
                        let weapon = self.weapons.get_mut(*weapon);
                        let (weapon_kind, ammo) = match kind {
                            ItemKind::Plasma => (WeaponKind::PlasmaRifle, 20),
                            ItemKind::Ak47Ammo => (WeaponKind::Ak47, 30),
                            ItemKind::M4Ammo => (WeaponKind::M4, 25),
                            _ => continue,
                        };
                        if weapon.get_kind() == weapon_kind {
                            weapon.add_ammo(ammo);
                            break;
                        }
                    }
                }
            }
        }
    }

    fn pickup_item(&mut self, engine: &mut Engine, actor: Handle<Actor>, item: Handle<Item>) {
        if self.actors.contains(actor) && self.items.contains(item) {
            let item = self.items.get_mut(item);

            self.events_sender
                .as_ref()
                .unwrap()
                .send(GameEvent::AddNotification {
                    text: format!("Actor picked up item {:?}", item.get_kind())
                }).unwrap();

            let scene = engine.scenes.get_mut(self.scene);
            let position = item.position(&mut scene.graph);
            item.pick_up();
            let kind = item.get_kind();
            self.play_sound(engine, "data/sounds/item_pickup.ogg", position);
            self.give_item(engine, actor, kind);
        }
    }

    fn create_projectile(&mut self,
                         engine: &mut Engine,
                         kind: ProjectileKind,
                         position: Vec3,
                         direction: Vec3,
                         initial_velocity: Vec3,
                         owner: Handle<Weapon>,
    ) {
        let scene = engine.scenes.get_mut(self.scene);
        let resource_manager = &mut engine.resource_manager;
        let projectile = Projectile::new(
            kind,
            resource_manager,
            scene,
            direction,
            position,
            owner,
            initial_velocity,
            self.events_sender.as_ref().unwrap().clone(),
        );
        self.projectiles.add(projectile);
    }

    fn play_sound<P: AsRef<Path>>(&self, engine: &mut Engine, path: P, position: Vec3) {
        let mut sound_context = engine.sound_context.lock().unwrap();
        let shot_buffer = engine.resource_manager.request_sound_buffer(path, false).unwrap();
        let shot_sound = SpatialSourceBuilder::new(
            GenericSourceBuilder::new(shot_buffer)
                .with_status(Status::Playing)
                .with_play_once(true)
                .build()
                .unwrap())
            .with_position(position)
            .build_source();
        sound_context.add_source(shot_sound);
    }

    fn shoot_weapon(&mut self, engine: &mut Engine, weapon_handle: Handle<Weapon>, initial_velocity: Vec3, time: GameTime) {
        if self.weapons.contains(weapon_handle) {
            let scene = engine.scenes.get_mut(self.scene);
            let weapon = self.weapons.get_mut(weapon_handle);
            if weapon.try_shoot(scene, time) {
                let kind = weapon.definition.projectile;
                let position = weapon.get_shot_position(&scene.graph);
                let direction = weapon.get_shot_direction(&scene.graph);
                self.create_projectile(engine, kind, position, direction, initial_velocity, weapon_handle);
            }
        }
    }

    fn show_weapon(&mut self, engine: &mut Engine, weapon_handle: Handle<Weapon>, state: bool) {
        let scene = engine.scenes.get_mut(self.scene);
        self.weapons.get(weapon_handle).set_visibility(state, &mut scene.graph)
    }

    fn find_suitable_spawn_point(&self, engine: &mut Engine) -> usize {
        // Find spawn point with least amount of enemies nearby.
        let scene = engine.scenes.get(self.scene);
        let mut index = 0;
        let mut max_distance = std::f32::MAX;
        for (i, pt) in self.spawn_points.iter().enumerate() {
            let mut sum_distance = 0.0;
            for actor in self.actors.iter() {
                let position = actor.character().get_position(&scene.physics);
                sum_distance += pt.position.distance(&position);
            }
            if sum_distance > max_distance {
                max_distance = sum_distance;
                index = i;
            }
        }
        index
    }

    fn spawn_bot(&mut self, engine: &mut Engine, kind: BotKind) {
        let index = self.find_suitable_spawn_point(engine);
        let spawn_position = self.spawn_points
            .get(index)
            .map_or(Vec3::ZERO, |pt| pt.position);
        self.add_bot(engine, kind, spawn_position);

        self.events_sender
            .as_ref()
            .unwrap()
            .send(GameEvent::AddNotification {
                text: format!("Bot {:?} spawned!", kind)
            }).unwrap();
    }

    fn damage_actor(&mut self, actor: Handle<Actor>, who: Handle<Actor>, amount: f32) {
        if self.actors.contains(actor) && (who.is_none() || who.is_some() && self.actors.contains(who)) {
            let message =
                if who.is_some() {
                    format!("{} dealt {} damage to {}!", self.actors.get(who).character().name,
                            amount, self.actors.get(actor).character().name)
                } else {
                    format!("{} took {} damage!", self.actors.get(actor).character().name, amount)
                };

            self.events_sender
                .as_ref()
                .unwrap()
                .send(GameEvent::AddNotification {
                    text: message,
                }).unwrap();

            let actor = self.actors.get_mut(actor);
            actor.character_mut().damage(amount);
            if let Actor::Bot(bot) = actor {
                if who.is_some() {
                    bot.set_target_actor(who)
                }
            }
        }
    }

    fn spawn_item(&mut self, engine: &mut Engine, kind: ItemKind, position: Vec3, adjust_height: bool, lifetime: Option<f32>) {
        let position = if adjust_height {
            self.pick(engine, position, position - Vec3::new(0.0, 1000.0, 0.0))
        } else {
            position
        };
        let scene = engine.scenes.get_mut(self.scene);
        let resource_manager = &mut engine.resource_manager;
        let mut item = Item::new(kind, position, scene, resource_manager, self.events_sender.as_ref().unwrap().clone());
        item.set_lifetime(lifetime);
        self.items.add(item);
    }

    pub fn update(&mut self, engine: &mut Engine, time: GameTime) {
        let scene = engine.scenes.get_mut(self.scene);

        let player_position = self.actors
            .get(self.player)
            .character()
            .get_position(&scene.physics);

        for (handle, actor) in self.actors.pair_iter_mut() {
            // TODO: Replace with automatic target selection.
            if let Actor::Bot(bot) = actor {
                bot.set_target(player_position);
            }

            for death_zone in self.death_zones.iter() {
                if death_zone.bounds.is_contains_point(actor.character().get_position(&scene.physics)) {
                    self.events_sender
                        .as_ref()
                        .unwrap()
                        .send(GameEvent::RespawnActor { actor: handle })
                        .unwrap();
                }
            }
        }

        self.weapons.update(scene);
        self.projectiles.update(scene, &mut self.actors, &self.weapons, time);
        self.items.update(scene, time);

        let mut context = LevelUpdateContext {
            time,
            scene,
            sound_context: engine.sound_context.clone(),
            items: &self.items,
            jump_pads: &self.jump_pads,
            navmesh: self.navmesh.as_mut(),
        };

        self.actors.update(&mut context);
    }

    pub fn respawn_actor(&mut self, engine: &mut Engine, actor: Handle<Actor>) {
        if self.actors.contains(actor) {
            let kind = match self.actors.get(actor) {
                Actor::Bot(bot) => Some(bot.definition.kind),
                Actor::Player(_) => None,
            };

            self.remove_actor(engine, actor);

            match kind {
                Some(bot) => {
                    // Spawn bot of same kind, we don't care of preserving state of bot
                    // after death. Leader board still will correctly count score.
                    self.spawn_bot(engine, bot)
                }
                None => self.spawn_player(engine)
            }
        }
    }

    pub fn handle_game_event(&mut self, engine: &mut Engine, event: &GameEvent, time: GameTime) {
        match event {
            GameEvent::GiveNewWeapon { actor, kind } => {
                self.give_new_weapon(engine, *actor, *kind);
            }
            GameEvent::AddBot { kind, position } => {
                self.add_bot(engine, *kind, *position)
            }
            GameEvent::RemoveActor { actor } => {
                self.remove_actor(engine, *actor)
            }
            GameEvent::GiveItem { actor, kind } => {
                self.give_item(engine, *actor, *kind);
            }
            GameEvent::PickUpItem { actor, item } => {
                self.pickup_item(engine, *actor, *item);
            }
            GameEvent::ShootWeapon { weapon, initial_velocity } => {
                self.shoot_weapon(engine, *weapon, *initial_velocity, time)
            }
            GameEvent::CreateProjectile { kind, position, direction, initial_velocity, owner } => {
                self.create_projectile(engine, *kind, *position, *direction, *initial_velocity, *owner)
            }
            GameEvent::PlaySound { path, position } => {
                self.play_sound(engine, path, *position)
            }
            GameEvent::ShowWeapon { weapon, state } => {
                self.show_weapon(engine, *weapon, *state)
            }
            GameEvent::SpawnBot { kind } => {
                self.spawn_bot(engine, *kind);
            }
            GameEvent::DamageActor { actor, who, amount } => {
                self.damage_actor(*actor, *who, *amount);
            }
            GameEvent::CreateEffect { kind, position } => {
                let scene = engine.scenes.get_mut(self.scene);
                effects::create(*kind, &mut scene.graph, &mut engine.resource_manager, *position)
            }
            GameEvent::SpawnPlayer => {
                self.spawn_player(engine)
            }
            GameEvent::SpawnItem { kind, position, adjust_height, lifetime } => {
                self.spawn_item(engine, *kind, *position, *adjust_height, *lifetime)
            }
            GameEvent::AddNotification { .. } => {
                // Ignore
            }
            GameEvent::RespawnActor { actor } => {
                self.respawn_actor(engine, *actor)
            }
        }
    }

    pub fn set_events_sender(&mut self, sender: Sender<GameEvent>) {
        self.events_sender = Some(sender.clone());

        // Attach new sender to all event sources.
        for actor in self.actors.iter_mut() {
            actor.character_mut().sender = Some(sender.clone());
        }
        for weapon in self.weapons.iter_mut() {
            weapon.sender = Some(sender.clone());
        }
        for projectile in self.projectiles.iter_mut() {
            projectile.sender = Some(sender.clone());
        }
    }

    pub fn debug_draw(&self, engine: &mut Engine) {
        let debug_renderer = &mut engine.renderer.debug_renderer;

        if let Some(navmesh) = self.navmesh.as_ref() {
            for pt in navmesh.vertices() {
                for neighbour in pt.neighbours() {
                    debug_renderer.add_line(debug_renderer::Line {
                        begin: pt.position(),
                        end: navmesh.vertices()[*neighbour].position(),
                        color: Default::default(),
                    });
                }
            }

            for actor in self.actors.iter() {
                if let Actor::Bot(bot) = actor {
                    bot.debug_draw(debug_renderer);
                }
            }
        }

        let scene = engine.scenes.get(self.scene);

        for death_zone in self.death_zones.iter() {
            debug_renderer.draw_aabb(&death_zone.bounds, Color::opaque(0, 0, 200));
        }
    }
}

pub struct SpawnPoint {
    position: Vec3
}

impl Default for SpawnPoint {
    fn default() -> Self {
        Self {
            position: Default::default()
        }
    }
}

impl Visit for SpawnPoint {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.position.visit("Position", visitor)?;

        visitor.leave_region()
    }
}

#[allow(dead_code)]
pub enum GameEvent {
    GiveNewWeapon {
        actor: Handle<Actor>,
        kind: WeaponKind,
    },
    AddBot {
        kind: BotKind,
        position: Vec3,
    },
    RemoveActor {
        actor: Handle<Actor>
    },
    /// Spawns new bot at random spawn point. Selection of spawn point can be based on some
    /// particular heuristic, leading to good selection (like do spawn at a point with least
    /// enemies nearby, which will increase survival probability)
    SpawnBot {
        kind: BotKind
    },
    /// Gives item of specified kind to a given actor. Basically it means that actor will take
    /// item and consume it immediately (heal itself, add ammo, etc.)
    GiveItem {
        actor: Handle<Actor>,
        kind: ItemKind,
    },
    PickUpItem {
        actor: Handle<Actor>,
        item: Handle<Item>,
    },
    SpawnItem {
        kind: ItemKind,
        position: Vec3,
        adjust_height: bool,
        lifetime: Option<f32>,
    },
    CreateProjectile {
        kind: ProjectileKind,
        position: Vec3,
        direction: Vec3,
        initial_velocity: Vec3,
        owner: Handle<Weapon>,
    },
    ShootWeapon {
        weapon: Handle<Weapon>,
        initial_velocity: Vec3,
    },
    PlaySound {
        path: PathBuf,
        position: Vec3,
    },
    ShowWeapon {
        weapon: Handle<Weapon>,
        state: bool,
    },
    DamageActor {
        actor: Handle<Actor>,
        /// Actor who damaged target actor, can be Handle::NONE if damage came from environment
        /// or not from any actor.
        who: Handle<Actor>,
        amount: f32,
    },
    CreateEffect {
        kind: EffectKind,
        position: Vec3,
    },
    SpawnPlayer,
    /// HUD listens such events and puts them into queue.
    AddNotification {
        text: String
    },
    RespawnActor {
        actor: Handle<Actor>
    },
}