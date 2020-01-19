use std::{
    path::Path,
    cell::RefCell,
    rc::Rc,
    sync::{
        Mutex,
        Arc,
        mpsc::{Sender, Receiver, channel},
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
};
use rg3d::{
    engine::{
        resource_manager::ResourceManager,
        Engine,
    },
    resource::{
        texture::TextureKind,
    },
    event::Event,
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
        transform::TransformBuilder,
    },
    utils,
    core::{
        color::Color,
        color_gradient::{ColorGradient, GradientPoint},
        pool::Handle,
        visitor::{
            Visit,
            VisitResult,
            Visitor,
        },
        math::vec3::*,
        math::ray::Ray,
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

pub struct Level {
    scene: Handle<Scene>,
    player: Handle<Actor>,
    projectiles: ProjectileContainer,
    actors: ActorContainer,
    weapons: WeaponContainer,
    jump_pads: JumpPadContainer,
    items: ItemContainer,
    spawn_points: Vec<SpawnPoint>,
    events_receiver: Receiver<GameEvent>,
    events_sender: Sender<GameEvent>,
}

impl Default for Level {
    fn default() -> Self {
        let (tx, rx) = channel();
        Self {
            projectiles: ProjectileContainer::new(),
            actors: ActorContainer::new(),
            scene: Handle::NONE,
            player: Handle::NONE,
            weapons: WeaponContainer::new(),
            jump_pads: JumpPadContainer::new(),
            items: ItemContainer::new(),
            spawn_points: Default::default(),
            events_receiver: rx,
            events_sender: tx,
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

        // Attach new sender to all event sources.
        for actor in self.actors.iter_mut() {
            actor.character_mut().sender = Some(self.events_sender.clone());
        }

        for weapon in self.weapons.iter_mut() {
            weapon.sender = Some(self.events_sender.clone());
        }

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

pub struct LevelUpdateContext<'a> {
    pub time: GameTime,
    pub scene: &'a mut Scene,
    pub sound_context: Arc<Mutex<Context>>,
    pub resource_manager: &'a mut ResourceManager,
    pub items: &'a ItemContainer,
    pub weapons: &'a WeaponContainer,
    pub jump_pads: &'a JumpPadContainer,
    pub spawn_points: &'a [SpawnPoint],
}

impl Level {
    fn load_level(engine: &mut Engine) -> Scene {
        let mut scene = Scene::new();

        let map_model = engine.resource_manager.request_model(Path::new("data/models/dm6.fbx"));
        if map_model.is_some() {
            // Instantiate map
            let map_root_handle = map_model.unwrap().lock().unwrap().instantiate(&mut scene).root;
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

        scene
    }

    pub fn new(engine: &mut Engine, control_scheme: Rc<RefCell<ControlScheme>>) -> Level {
        let mut scene = Self::load_level(engine);
        let mut actors = ActorContainer::new();
        let (tx, rx) = channel();
        let mut player = Player::new(engine.sound_context.clone(), &mut engine.resource_manager, &mut scene, tx.clone());
        player.set_control_scheme(control_scheme);
        let player = actors.add(Actor::Player(player));
        let mut level = Level {
            weapons: WeaponContainer::new(),
            actors,
            player,
            scene: engine.scenes.add(scene),
            events_receiver: rx,
            events_sender: tx,
            ..Default::default()
        };
        level.events_sender.send(GameEvent::GiveNewWeapon { actor: player, kind: WeaponKind::M4 }).unwrap();
        level.events_sender.send(GameEvent::GiveNewWeapon { actor: player, kind: WeaponKind::Ak47 }).unwrap();
        level.events_sender.send(GameEvent::GiveNewWeapon { actor: player, kind: WeaponKind::PlasmaRifle }).unwrap();
        level.events_sender.send(GameEvent::AddBot { kind: BotKind::Maw, position: Vec3::new(0.0, 0.0, -1.0) }).unwrap();
        level.events_sender.send(GameEvent::AddBot { kind: BotKind::Mutant, position: Vec3::new(0.0, 0.0, 1.0) }).unwrap();
        level.events_sender.send(GameEvent::AddBot { kind: BotKind::Parasite, position: Vec3::new(1.0, 0.0, 0.0) }).unwrap();
        level.analyze(engine);
        level
    }

    pub fn analyze(&mut self, engine: &mut Engine) {
        let mut items = Vec::new();
        let mut spawn_points = Vec::new();
        let scene = engine.scenes.get_mut(self.scene);
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
            } else if name.starts_with("SpawnPoint") {
                spawn_points.push(node.base().get_global_position())
            }
        }
        for (kind, position) in items {
            self.items.add(Item::new(kind, position, scene, &mut engine.resource_manager));
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

    fn update_bots(&mut self, engine: &mut Engine) {
        for actor in self.actors.iter_mut() {
            if let Actor::Bot(bot) = actor {}
        }
    }

    fn pick(&self, engine: &mut Engine, from: Vec3, to: Vec3) -> Vec3 {
        let scene = engine.scenes.get_mut(self.scene);
        let SceneInterfaceMut { physics, .. } = scene.interface_mut();
        if let Some(ray) = Ray::from_two_points(&from, &to) {
            let mut intersections = Vec::new();
            let options = RayCastOptions { ignore_bodies: true, ..Default::default() };
            physics.ray_cast(&ray, options, &mut intersections);
            if let Some(pt) = intersections.first() {
                pt.position
            } else {
                from
            }
        } else {
            from
        }
    }

    fn drop_weapon(&mut self, engine: &mut Engine, weapon: Handle<Weapon>, position: Vec3, adjust_height: bool) {
        let position = if adjust_height {
            self.pick(engine, position, position - Vec3::new(0.0, 1000.0, 0.0))
        } else {
            position
        };
        let scene = engine.scenes.get_mut(self.scene);
        let SceneInterfaceMut { physics, graph, .. } = scene.interface_mut();
        let weapon = self.weapons.get_mut(weapon);
        weapon.set_owner(Handle::NONE);
        graph.unlink_nodes(weapon.get_model());
        graph.get_mut(weapon.get_model())
            .base_mut()
            .get_local_transform_mut()
            .set_position(position);
    }

    fn spawn_weapon(&mut self, engine: &mut Engine, kind: WeaponKind, position: Vec3, adjust_height: bool) {
        let position = if adjust_height {
            self.pick(engine, position, position - Vec3::new(0.0, 1000.0, 0.0))
        } else {
            position
        };
        let scene = engine.scenes.get_mut(self.scene);
        let weapon = Weapon::new(kind, &mut engine.resource_manager, scene, self.events_sender.clone());
        scene.interface_mut()
            .graph
            .get_mut(weapon.get_model())
            .base_mut()
            .get_local_transform_mut()
            .set_position(position);
        self.weapons.add(weapon);
    }

    fn give_new_weapon(&mut self, engine: &mut Engine, actor: Handle<Actor>, kind: WeaponKind) {
        let scene = engine.scenes.get_mut(self.scene);
        let mut weapon = Weapon::new(kind, &mut engine.resource_manager, scene, self.events_sender.clone());
        weapon.set_owner(actor);
        let weapon_model = weapon.get_model();
        let actor = self.actors.get_mut(actor);
        let weapon_handle = self.weapons.add(weapon);
        actor.character_mut().add_weapon(weapon_handle);
        let SceneInterfaceMut { graph, .. } = scene.interface_mut();
        graph.link_nodes(weapon_model, actor.character().get_weapon_pivot());
    }

    fn add_bot(&mut self, engine: &mut Engine, kind: BotKind, position: Vec3) {
        let scene = engine.scenes.get_mut(self.scene);
        let bot = Actor::Bot(Bot::new(kind, &mut engine.resource_manager, scene, position, self.events_sender.clone()).unwrap());
        let bot = self.actors.add(bot);
        self.events_sender.send(GameEvent::GiveNewWeapon { actor: bot, kind: WeaponKind::Ak47 }).unwrap();
        println!("Bot {:?} was added!", kind);
    }

    fn remove_actor(&mut self, engine: &mut Engine, actor: Handle<Actor>) {
        let scene = engine.scenes.get_mut(self.scene);
        let character = self.actors.get(actor).character();

        // Make sure to drop weapons
        let drop_position = character.get_position(scene.interface_mut().physics);
        let weapons = character.get_weapons()
            .iter()
            .map(|h| *h)
            .collect::<Vec<Handle<Weapon>>>();
        for weapon in weapons {
            self.drop_weapon(engine, weapon, drop_position, true);
        }

        let scene = engine.scenes.get_mut(self.scene);
        self.actors.get_mut(actor).character_mut().clean_up(scene);
        self.actors.free(actor)
    }

    fn give_item(&mut self, engine: &mut Engine, actor: Handle<Actor>, kind: ItemKind) {
        let character = self.actors.get_mut(actor).character_mut();
        match kind {
            ItemKind::Medkit => character.heal(20.0),
            ItemKind::Plasma | ItemKind::Ak47Ammo762 | ItemKind::M4Ammo556 => {
                for weapon in character.get_weapons() {
                    let weapon = self.weapons.get_mut(*weapon);
                    let (weapon_kind, ammo) = match kind {
                        ItemKind::Medkit => continue,
                        ItemKind::Plasma => (WeaponKind::PlasmaRifle, 20),
                        ItemKind::Ak47Ammo762 => (WeaponKind::Ak47, 30),
                        ItemKind::M4Ammo556 => (WeaponKind::M4, 25),
                    };
                    if weapon.get_kind() == weapon_kind {
                        weapon.add_ammo(ammo);
                        break;
                    }
                }
            }
        }
    }

    fn pickup_item(&mut self, engine: &mut Engine, actor: Handle<Actor>, item: Handle<Item>) {
        let item = self.items.get_mut(item);
        let scene = engine.scenes.get_mut(self.scene);
        let graph = scene.interface().graph;
        let position = item.position(graph);
        item.pick_up();
        let kind = item.get_kind();
        self.play_sound(engine, "data/sounds/item_pickup.ogg", position);
        self.give_item(engine, actor, kind);
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
        let scene = engine.scenes.get_mut(self.scene);
        let weapon = self.weapons.get_mut(weapon_handle);
        if weapon.try_shoot(scene, time, initial_velocity) {
            let graph = scene.interface().graph;
            let kind = weapon.definition.projectile;
            let position = weapon.get_shot_position(graph);
            let direction = weapon.get_shot_direction(graph);
            self.create_projectile(engine, kind, position, direction, initial_velocity, weapon_handle);
        }
    }

    fn show_weapon(&mut self, engine: &mut Engine, weapon_handle: Handle<Weapon>, state: bool) {
        let scene = engine.scenes.get_mut(self.scene);
        self.weapons.get(weapon_handle).set_visibility(state, scene.interface_mut().graph)
    }

    fn spawn_bot(&mut self, engine: &mut Engine, kind: BotKind) {
        // Find spawn point with least amount of enemies nearby.
        let scene = engine.scenes.get(self.scene);
        let physics = scene.interface().physics;
        let mut index = 0;
        let mut max_distance = std::f32::MAX;
        for (i, pt) in self.spawn_points.iter().enumerate() {
            let mut sum_distance = 0.0;
            for actor in self.actors.iter() {
                let position = actor.character().get_position(physics);
                sum_distance += pt.position.distance(&position);
            }
            if sum_distance > max_distance {
                max_distance = sum_distance;
                index = i;
            }
        }

        let spawn_position = self.spawn_points.get(index).map_or(Vec3::ZERO, |pt| pt.position);

        self.add_bot(engine, kind, spawn_position);
    }

    fn damage_actor(&mut self, actor: Handle<Actor>, who: Handle<Actor>, amount: f32) {
        let actor = self.actors.get_mut(actor);
        actor.character_mut().damage(amount);
        if let Actor::Bot(bot) = actor {
            if who.is_some() {
                bot.set_target_actor(who)
            }
        }
    }

    pub fn update(&mut self, engine: &mut Engine, time: GameTime) {
        let scene = engine.scenes.get_mut(self.scene);

        let player_position = self.actors.get(self.player).character().get_position(scene.interface().physics);

        for actor in self.actors.iter_mut() {
            if let Actor::Bot(bot) = actor {
                bot.set_target(player_position);
            }
        }

        self.weapons.update(scene);
        self.projectiles.update(scene, &mut engine.resource_manager, &mut self.actors, &self.weapons, time);
        self.items.update(scene, &mut engine.resource_manager, time);

        let mut context = LevelUpdateContext {
            time,
            scene,
            sound_context: engine.sound_context.clone(),
            resource_manager: &mut engine.resource_manager,
            items: &self.items,
            weapons: &self.weapons,
            jump_pads: &self.jump_pads,
            spawn_points: &self.spawn_points,
        };

        self.actors.update(&mut context);

        while let Ok(event) = self.events_receiver.try_recv() {
            match event {
                GameEvent::SpawnWeapon { kind, position, adjust_height } => {
                    self.spawn_weapon(engine, kind, position, adjust_height)
                }
                GameEvent::DropWeapon { weapon, position, adjust_height } => {
                    self.drop_weapon(engine, weapon, position, adjust_height)
                }
                GameEvent::GiveNewWeapon { actor, kind } => {
                    self.give_new_weapon(engine, actor, kind)
                }
                GameEvent::AddBot { kind, position } => {
                    self.add_bot(engine, kind, position)
                }
                GameEvent::RemoveActor { actor } => {
                    self.remove_actor(engine, actor)
                }
                GameEvent::GiveItem { actor, kind } => {
                    self.give_item(engine, actor, kind)
                }
                GameEvent::PickUpItem { actor, item } => {
                    self.pickup_item(engine, actor, item)
                }
                GameEvent::ShootWeapon { weapon, initial_velocity } => {
                    self.shoot_weapon(engine, weapon, initial_velocity, time)
                }
                GameEvent::CreateProjectile { kind, position, direction, initial_velocity, owner } => {
                    self.create_projectile(engine, kind, position, direction, initial_velocity, owner)
                }
                GameEvent::PlaySound { path, position } => {
                    self.play_sound(engine, path, position)
                }
                GameEvent::ShowWeapon { weapon, state } => {
                    self.show_weapon(engine, weapon, state)
                }
                GameEvent::SpawnBot { kind } => {
                    self.spawn_bot(engine, kind)
                }
                GameEvent::DamageActor { actor, who, amount } => {
                    self.damage_actor(actor, who, amount)
                }
            }
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

pub enum GameEvent {
    SpawnWeapon {
        kind: WeaponKind,
        position: Vec3,
        adjust_height: bool,
    },
    DropWeapon {
        weapon: Handle<Weapon>,
        position: Vec3,
        adjust_height: bool,
    },
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
        amount: f32
    }
}