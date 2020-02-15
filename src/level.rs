use std::{
    path::Path,
    cell::RefCell,
    rc::Rc,
    sync::{
        Mutex,
        Arc,
        mpsc::Sender,
    },
    collections::HashMap,
};
use rand::Rng;
use crate::{actor::{ActorContainer, Actor}, weapon::{
    Weapon,
    WeaponKind,
    WeaponContainer,
}, player::Player, GameTime, bot::{
    Bot,
    BotKind,
}, projectile::{
    ProjectileContainer,
    ProjectileKind,
    Projectile,
}, character::AsCharacter, jump_pad::{JumpPadContainer, JumpPad}, item::{ItemContainer, Item, ItemKind}, control_scheme::ControlScheme, effects, message::Message, MatchOptions, GameEngine};
use rg3d::{
    core::math::PositionProvider,
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
    renderer::debug_renderer,
};
use crate::character::Team;

pub struct Level {
    map_root: Handle<Node>,
    pub scene: Handle<Scene>,
    player: Handle<Actor>,
    projectiles: ProjectileContainer,
    pub actors: ActorContainer,
    weapons: WeaponContainer,
    jump_pads: JumpPadContainer,
    items: ItemContainer,
    spawn_points: Vec<SpawnPoint>,
    sender: Option<Sender<Message>>,
    pub navmesh: Option<Navmesh>,
    pub control_scheme: Option<Rc<RefCell<ControlScheme>>>,
    death_zones: Vec<DeathZone>,
    pub options: MatchOptions,
    time: f32,
    pub leader_board: LeaderBoard,
}

#[derive(Copy, Clone)]
pub struct PersonalScore {
    pub kills: u32,
    pub deaths: u32,
}

impl Default for PersonalScore {
    fn default() -> Self {
        Self {
            kills: 0,
            deaths: 0,
        }
    }
}

impl Visit for PersonalScore {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.kills.visit("Kills", visitor)?;
        self.deaths.visit("Deaths", visitor)?;

        visitor.leave_region()
    }
}

pub struct LeaderBoard {
    personal_score: HashMap<String, PersonalScore>,
    team_score: HashMap<Team, u32>,
}

impl LeaderBoard {
    pub fn get_or_add_actor<P: AsRef<str>>(&mut self, actor_name: P) -> &mut PersonalScore {
        self.personal_score
            .entry(actor_name.as_ref().to_owned())
            .or_insert(Default::default())
    }

    pub fn remove_actor<P: AsRef<str>>(&mut self, actor_name: P) {
        self.personal_score.remove(actor_name.as_ref());
    }

    pub fn add_frag<P: AsRef<str>>(&mut self, actor_name: P) {
        self.get_or_add_actor(actor_name).kills += 1;
    }

    pub fn add_death<P: AsRef<str>>(&mut self, actor_name: P) {
        self.get_or_add_actor(actor_name).deaths += 1;
    }

    pub fn score_of<P: AsRef<str>>(&self, actor_name: P) -> u32 {
        match self.personal_score.get(actor_name.as_ref()) {
            None => 0,
            Some(value) => value.kills,
        }
    }

    pub fn add_team_frag(&mut self, team: Team) {
        *self.team_score.entry(team).or_insert(0) += 1;
    }

    pub fn team_score(&self, team: Team) -> u32 {
        match self.team_score.get(&team) {
            None => 0,
            Some(score) => *score,
        }
    }

    pub fn highest_personal_score(&self) -> Option<(&str, u32)> {
        let mut pair = None;

        for (name, score) in self.personal_score.iter() {
            match pair {
                None => pair = Some((name.as_str(), score.kills)),
                Some(ref mut pair) => {
                    if score.kills > pair.1 {
                        *pair = (name.as_str(), score.kills)
                    }
                }
            }
        }

        pair
    }

    pub fn values(&self) -> &HashMap<String, PersonalScore> {
        &self.personal_score
    }
}

impl Default for LeaderBoard {
    fn default() -> Self {
        Self {
            personal_score: Default::default(),
            team_score: Default::default(),
        }
    }
}

impl Visit for LeaderBoard {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.personal_score.visit("PersonalScore", visitor)?;
        self.team_score.visit("TeamScore", visitor)?;

        visitor.leave_region()
    }
}

impl Default for Level {
    fn default() -> Self {
        Self {
            map_root: Default::default(),
            projectiles: ProjectileContainer::new(),
            actors: ActorContainer::new(),
            scene: Handle::NONE,
            player: Handle::NONE,
            weapons: WeaponContainer::new(),
            jump_pads: JumpPadContainer::new(),
            items: ItemContainer::new(),
            spawn_points: Default::default(),
            sender: None,
            navmesh: Default::default(),
            control_scheme: None,
            death_zones: Default::default(),
            options: Default::default(),
            time: 0.0,
            leader_board: Default::default(),
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
        self.map_root.visit("MapRoot", visitor)?;
        self.player.visit("Player", visitor)?;
        self.actors.visit("Actors", visitor)?;
        self.projectiles.visit("Projectiles", visitor)?;
        self.weapons.visit("Weapons", visitor)?;
        self.jump_pads.visit("JumpPads", visitor)?;
        self.spawn_points.visit("SpawnPoints", visitor)?;
        self.death_zones.visit("DeathZones", visitor)?;
        self.options.visit("Options", visitor)?;
        self.time.visit("Time", visitor)?;
        self.leader_board.visit("LeaderBoard", visitor)?;

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
    pub fn new(
        engine: &mut GameEngine,
        control_scheme: Rc<RefCell<ControlScheme>>,
        sender: Sender<Message>,
        options: MatchOptions,
    ) -> Level {
        let mut scene = Scene::new();

        let mut map_root_handle = Handle::NONE;
        let map_model = engine.resource_manager.request_model(Path::new("data/models/dm6.fbx"));
        if map_model.is_some() {
            // Instantiate map
            map_root_handle = map_model.unwrap().lock().unwrap().instantiate_geometry(&mut scene);
            // Create collision geometry
            let polygon_handle = scene.graph.find_by_name(map_root_handle, "Polygon");
            if polygon_handle.is_some() {
                scene.physics.add_static_geometry(utils::mesh_to_static_geometry(scene.graph.get(polygon_handle).as_mesh()));
            } else {
                println!("Unable to find Polygon node to build collision shape for level!");
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

        let mut level = Level {
            scene: engine.scenes.add(scene),
            sender: Some(sender.clone()),
            control_scheme: Some(control_scheme),
            map_root: map_root_handle,
            options,
            ..Default::default()
        };

        level.build_navmesh(engine);
        level.analyze(engine);

        sender.send(Message::SpawnPlayer).unwrap();
        sender.send(Message::AddBot { kind: BotKind::Maw, position: Vec3::new(0.0, 0.0, -1.0), name: None }).unwrap();
        sender.send(Message::AddBot { kind: BotKind::Mutant, position: Vec3::new(0.0, 0.0, 1.0), name: None }).unwrap();
        sender.send(Message::AddBot { kind: BotKind::Parasite, position: Vec3::new(1.0, 0.0, 0.0), name: None }).unwrap();

        level
    }

    pub fn build_navmesh(&mut self, engine: &mut GameEngine) {
        if self.navmesh.is_none() {
            let scene = engine.scenes.get_mut(self.scene);
            let navmesh_handle = scene.graph.find_by_name(self.map_root, "Navmesh");
            if navmesh_handle.is_some() {
                let navmesh_node = scene.graph.get_mut(navmesh_handle);
                navmesh_node.base_mut().set_visibility(false);
                self.navmesh = Some(utils::mesh_to_navmesh(navmesh_node.as_mesh()));
            } else {
                println!("Unable to find Navmesh node to build navmesh!")
            }
        }
    }

    pub fn analyze(&mut self, engine: &mut GameEngine) {
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
                if let Node::Mesh(_) = node {
                    death_zones.push(handle);
                }
            }
        }
        for (kind, position) in items {
            self.items.add(Item::new(kind, position, scene, &mut engine.resource_manager, self.sender.as_ref().unwrap().clone()));
        }
        for handle in death_zones {
            let node = scene.graph.get_mut(handle);
            node.base_mut().set_visibility(false);
            self.death_zones
                .push(DeathZone {
                    bounds: node.as_mesh()
                        .calculate_world_bounding_box()
                });
        }
        self.spawn_points = spawn_points
            .into_iter()
            .map(|p| SpawnPoint { position: p })
            .collect();
    }

    pub fn destroy(&mut self, engine: &mut GameEngine) {
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

    fn pick(&self, engine: &mut GameEngine, from: Vec3, to: Vec3) -> Vec3 {
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

    fn remove_weapon(&mut self, engine: &mut GameEngine, weapon: Handle<Weapon>) {
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

    fn give_new_weapon(&mut self, engine: &mut GameEngine, actor: Handle<Actor>, kind: WeaponKind) {
        if self.actors.contains(actor) {
            let scene = engine.scenes.get_mut(self.scene);
            let mut weapon = Weapon::new(kind, &mut engine.resource_manager, scene, self.sender.as_ref().unwrap().clone());
            weapon.set_owner(actor);
            let weapon_model = weapon.get_model();
            let actor = self.actors.get_mut(actor);
            let weapon_handle = self.weapons.add(weapon);
            actor.character_mut().add_weapon(weapon_handle);
            scene.graph.link_nodes(weapon_model, actor.character().get_weapon_pivot());

            self.sender
                .as_ref()
                .unwrap()
                .send(Message::AddNotification {
                    text: format!("Actor picked up weapon {:?}", kind)
                }).unwrap();
        }
    }

    fn add_bot(&mut self, engine: &mut GameEngine, kind: BotKind, position: Vec3, name: Option<String>) -> Handle<Actor> {
        let scene = engine.scenes.get_mut(self.scene);
        let mut bot = Bot::new(kind, &mut engine.resource_manager, scene, position, self.sender.as_ref().unwrap().clone()).unwrap();
        let name = name.unwrap_or_else(|| format!("Bot {:?} {}", kind, self.actors.count()));
        self.leader_board.get_or_add_actor(&name);
        bot.set_target_actor(self.player)
            .character_mut()
            .set_name(name);
        let bot = self.actors.add(Actor::Bot(bot));
        self.give_new_weapon(engine, bot, WeaponKind::Ak47);
        bot
    }

    fn remove_actor(&mut self, engine: &mut GameEngine, actor: Handle<Actor>) {
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

    fn spawn_player(&mut self, engine: &mut GameEngine) -> Handle<Actor> {
        let index = self.find_suitable_spawn_point(engine);
        let spawn_position = self.spawn_points
            .get(index)
            .map_or(Vec3::ZERO, |pt| pt.position);
        let scene = engine.scenes.get_mut(self.scene);
        let mut player = Player::new(scene, self.sender.as_ref().unwrap().clone());
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
        self.player
    }

    fn give_item(&mut self, engine: &mut GameEngine, actor: Handle<Actor>, kind: ItemKind) {
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

    fn pickup_item(&mut self, engine: &mut GameEngine, actor: Handle<Actor>, item: Handle<Item>) {
        if self.actors.contains(actor) && self.items.contains(item) {
            let item = self.items.get_mut(item);

            self.sender
                .as_ref()
                .unwrap()
                .send(Message::AddNotification {
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
                         engine: &mut GameEngine,
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
            self.sender.as_ref().unwrap().clone(),
        );
        self.projectiles.add(projectile);
    }

    fn play_sound<P: AsRef<Path>>(&self, engine: &mut GameEngine, path: P, position: Vec3) {
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

    fn shoot_weapon(&mut self, engine: &mut GameEngine, weapon_handle: Handle<Weapon>, initial_velocity: Vec3, time: GameTime) {
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

    fn show_weapon(&mut self, engine: &mut GameEngine, weapon_handle: Handle<Weapon>, state: bool) {
        let scene = engine.scenes.get_mut(self.scene);
        self.weapons.get(weapon_handle).set_visibility(state, &mut scene.graph)
    }

    fn find_suitable_spawn_point(&self, engine: &mut GameEngine) -> usize {
        // Find spawn point with least amount of enemies nearby.
        let scene = engine.scenes.get(self.scene);
        let mut index = rand::thread_rng().gen_range(0, self.spawn_points.len());
        let mut max_distance = -std::f32::MAX;
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

    fn spawn_bot(&mut self, engine: &mut GameEngine, kind: BotKind, name: Option<String>) -> Handle<Actor> {
        let index = self.find_suitable_spawn_point(engine);
        let spawn_position = self.spawn_points
            .get(index)
            .map_or(Vec3::ZERO, |pt| pt.position);

        let bot = self.add_bot(engine, kind, spawn_position, name);

        self.sender
            .as_ref()
            .unwrap()
            .send(Message::AddNotification {
                text: format!("Bot {} spawned!", self.actors.get(bot).character().name)
            }).unwrap();

        bot
    }

    fn damage_actor(&mut self, actor: Handle<Actor>, who: Handle<Actor>, amount: f32) {
        if self.actors.contains(actor) && (who.is_none() || who.is_some() && self.actors.contains(who)) {
            let mut who_name = Default::default();
            let message =
                if who.is_some() {
                    who_name = self.actors.get(who).character().name.clone();
                    format!("{} dealt {} damage to {}!", who_name, amount, self.actors.get(actor).character().name)
                } else {
                    format!("{} took {} damage!", self.actors.get(actor).character().name, amount)
                };


            self.sender
                .as_ref()
                .unwrap()
                .send(Message::AddNotification {
                    text: message,
                }).unwrap();

            let actor = self.actors.get_mut(actor);
            let was_dead = actor.character().is_dead();
            actor.character_mut().damage(amount);
            if !was_dead && actor.character().is_dead() {
                if who.is_some() {
                    self.leader_board.add_frag(who_name)
                }
            }
            if let Actor::Bot(bot) = actor {
                if who.is_some() {
                    bot.set_target_actor(who);
                }
            }
        }
    }

    fn spawn_item(&mut self, engine: &mut GameEngine, kind: ItemKind, position: Vec3, adjust_height: bool, lifetime: Option<f32>) {
        let position = if adjust_height {
            self.pick(engine, position, position - Vec3::new(0.0, 1000.0, 0.0))
        } else {
            position
        };
        let scene = engine.scenes.get_mut(self.scene);
        let resource_manager = &mut engine.resource_manager;
        let mut item = Item::new(kind, position, scene, resource_manager, self.sender.as_ref().unwrap().clone());
        item.set_lifetime(lifetime);
        self.items.add(item);
    }

    pub fn time(&self) -> f32 {
        self.time
    }

    pub fn update(&mut self, engine: &mut GameEngine, time: GameTime) {
        self.time += time.delta;

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
                    self.sender
                        .as_ref()
                        .unwrap()
                        .send(Message::RespawnActor { actor: handle })
                        .unwrap();
                }
            }
        }

        self.weapons.update(scene, &self.actors);
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

    pub fn respawn_actor(&mut self, engine: &mut GameEngine, actor: Handle<Actor>) {
        if self.actors.contains(actor) {
            let name = self.actors.get(actor).character().name.clone();

            let kind = match self.actors.get(actor) {
                Actor::Bot(bot) => Some(bot.definition.kind),
                Actor::Player(_) => None,
            };

            self.remove_actor(engine, actor);

            self.leader_board.add_death(&name);

            match kind {
                Some(bot) => {
                    // Spawn bot of same kind, we don't care of preserving state of bot
                    // after death. Leader board still will correctly count score.
                    self.spawn_bot(engine, bot, Some(name))
                }
                None => self.spawn_player(engine)
            };
        }
    }

    pub fn handle_message(&mut self, engine: &mut GameEngine, message: &Message, time: GameTime) {
        match message {
            Message::GiveNewWeapon { actor, kind } => {
                self.give_new_weapon(engine, *actor, *kind);
            }
            Message::AddBot { kind, position, name } => {
                self.add_bot(engine, *kind, *position, name.clone());
            }
            Message::RemoveActor { actor } => {
                self.remove_actor(engine, *actor)
            }
            Message::GiveItem { actor, kind } => {
                self.give_item(engine, *actor, *kind);
            }
            Message::PickUpItem { actor, item } => {
                self.pickup_item(engine, *actor, *item);
            }
            Message::ShootWeapon { weapon, initial_velocity } => {
                self.shoot_weapon(engine, *weapon, *initial_velocity, time)
            }
            Message::CreateProjectile { kind, position, direction, initial_velocity, owner } => {
                self.create_projectile(engine, *kind, *position, *direction, *initial_velocity, *owner)
            }
            Message::PlaySound { path, position } => {
                self.play_sound(engine, path, *position)
            }
            Message::ShowWeapon { weapon, state } => {
                self.show_weapon(engine, *weapon, *state)
            }
            Message::SpawnBot { kind } => {
                self.spawn_bot(engine, *kind, None);
            }
            Message::DamageActor { actor, who, amount } => {
                self.damage_actor(*actor, *who, *amount);
            }
            Message::CreateEffect { kind, position } => {
                let scene = engine.scenes.get_mut(self.scene);
                effects::create(*kind, &mut scene.graph, &mut engine.resource_manager, *position)
            }
            Message::SpawnPlayer => {
                self.spawn_player(engine);
            }
            Message::SpawnItem { kind, position, adjust_height, lifetime } => {
                self.spawn_item(engine, *kind, *position, *adjust_height, *lifetime)
            }
            Message::RespawnActor { actor } => {
                self.respawn_actor(engine, *actor)
            }
            _ => ()
        }
    }

    pub fn set_message_sender(&mut self, sender: Sender<Message>) {
        self.sender = Some(sender.clone());

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

    pub fn debug_draw(&self, engine: &mut GameEngine) {
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