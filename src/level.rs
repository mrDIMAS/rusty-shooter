use std::{
    path::Path,
    cell::RefCell,
    rc::Rc,
    sync::{
        Mutex,
        Arc,
        mpsc::Sender,
    },
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
    jump_pad::{JumpPadContainer, JumpPad},
    item::{ItemContainer, Item, ItemKind},
    control_scheme::ControlScheme,
    effects,
    message::Message,
    MatchOptions,
    GameEngine,
    character::AsCharacter,
    leader_board::LeaderBoard,
};
use rg3d::{
    core::{
        math::{
            PositionProvider,
            vec3::*,
            ray::Ray,
            aabb::AxisAlignedBoundingBox,
        },
        color::Color,
        pool::Handle,
        visitor::{
            Visit,
            VisitResult,
            Visitor,
        },
    },
    event::Event,
    scene::{
        Scene,
        base::{
            AsBase,
            BaseBuilder,
        },
        node::Node,
        camera::CameraBuilder,
    },
    utils::{
        self,
        navmesh::Navmesh,
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

pub const RESPAWN_TIME: f32 = 4.0;

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
    respawn_list: Vec<RespawnEntry>,
    spectator_camera: Handle<Node>,
    target_spectator_position: Vec3,
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
            respawn_list: Default::default(),
            spectator_camera: Default::default(),
            target_spectator_position: Default::default(),
        }
    }
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
        self.respawn_list.visit("RespawnList", visitor)?;
        self.spectator_camera.visit("SpectatorCamera", visitor)?;
        self.target_spectator_position.visit("TargetSpectatorPosition", visitor)?;

        visitor.leave_region()
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

pub struct UpdateContext<'a> {
    pub time: GameTime,
    pub scene: &'a mut Scene,
    pub sound_context: Arc<Mutex<Context>>,
    pub items: &'a ItemContainer,
    pub jump_pads: &'a JumpPadContainer,
    pub navmesh: Option<&'a mut Navmesh>,
    pub weapons: &'a WeaponContainer,
}

struct PlayerRespawnEntry {
    time_left: f32
}

impl Default for PlayerRespawnEntry {
    fn default() -> Self {
        Self {
            time_left: 0.0
        }
    }
}

impl Visit for PlayerRespawnEntry {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.time_left.visit("TimeLeft", visitor)?;

        visitor.leave_region()
    }
}

struct BotRespawnEntry {
    name: String,
    kind: BotKind,
    time_left: f32,
}

impl Default for BotRespawnEntry {
    fn default() -> Self {
        Self {
            name: "".to_string(),
            kind: BotKind::Mutant,
            time_left: 0.0,
        }
    }
}

impl Visit for BotRespawnEntry {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.name.visit("Name", visitor)?;
        self.time_left.visit("TimeLeft", visitor)?;

        let mut kind_id = self.kind.id();
        kind_id.visit("Kind", visitor)?;
        self.kind = BotKind::from_id(kind_id)?;

        visitor.leave_region()
    }
}

enum RespawnEntry {
    Bot(BotRespawnEntry),
    Player(PlayerRespawnEntry),
}

impl Default for RespawnEntry {
    fn default() -> Self {
        RespawnEntry::Player(PlayerRespawnEntry::default())
    }
}

impl RespawnEntry {
    fn id(&self) -> u32 {
        match self {
            RespawnEntry::Bot { .. } => 0,
            RespawnEntry::Player { .. } => 1,
        }
    }

    fn from_id(id: u32) -> Result<Self, String> {
        match id {
            0 => Ok(RespawnEntry::Bot(Default::default())),
            1 => Ok(RespawnEntry::Player(Default::default())),
            _ => Err(format!("Invalid RespawnEntry type {}", id))
        }
    }
}

impl Visit for RespawnEntry {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        let mut id = self.id();
        id.visit("Id", visitor)?;
        if visitor.is_reading() {
            *self = Self::from_id(id)?;
        }

        match self {
            RespawnEntry::Bot(v) => v.visit("Data", visitor)?,
            RespawnEntry::Player(v) => v.visit("Data", visitor)?,
        }

        visitor.leave_region()
    }
}

impl Level {
    pub fn new(
        engine: &mut GameEngine,
        control_scheme: Rc<RefCell<ControlScheme>>,
        sender: Sender<Message>,
        options: MatchOptions,
    ) -> Level {
        let mut scene = Scene::new();

        // Spectator camera is used when there is no player on level.
        // This includes situation when player is dead - all dead actors are removed
        // from level.
        let spectator_camera = scene.graph.add_node(Node::Camera(
            CameraBuilder::new(BaseBuilder::new())
                .enabled(false)
                .build())
        );

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

        let mut level = Level {
            scene: engine.scenes.add(scene),
            sender: Some(sender.clone()),
            control_scheme: Some(control_scheme),
            map_root: map_root_handle,
            options,
            spectator_camera,
            ..Default::default()
        };

        level.build_navmesh(engine);
        level.analyze(engine);

        sender.send(Message::SpawnPlayer).unwrap();
        sender.send(Message::SpawnBot { kind: BotKind::Maw, name: "Maw".to_owned() }).unwrap();
        sender.send(Message::SpawnBot { kind: BotKind::Mutant, name: "Mutant".to_owned() }).unwrap();
        sender.send(Message::SpawnBot { kind: BotKind::Parasite, name: "Parasite".to_owned() }).unwrap();

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
        if self.player.is_some() {
            if let Actor::Player(player) = self.actors.get_mut(self.player) {
                return player.process_input_event(event);
            }
        }
        false
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
            scene.graph.link_nodes(weapon_model, actor.character().weapon_pivot());

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
        let bot = Bot::new(kind, &mut engine.resource_manager, scene, position, self.sender.as_ref().unwrap().clone()).unwrap();
        let name = name.unwrap_or_else(|| format!("Bot {:?} {}", kind, self.actors.count()));
        self.leader_board.get_or_add_actor(&name);
        let bot = self.actors.add(Actor::Bot(bot));
        self.give_new_weapon(engine, bot, WeaponKind::Ak47);
        bot
    }

    fn remove_actor(&mut self, engine: &mut GameEngine, actor: Handle<Actor>) {
        if self.actors.contains(actor) {
            let scene = engine.scenes.get_mut(self.scene);
            let character = self.actors.get(actor).character();

            // Make sure to remove weapons and drop appropriate items (items will be temporary).
            let drop_position = character.position(&scene.physics);
            let weapons = character.weapons()
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
        if let Node::Camera(spectator_camera) = scene.graph.get_mut(self.spectator_camera) {
            spectator_camera.set_enabled(false);
        }
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
                    for weapon_handle in character.weapons() {
                        let weapon = self.weapons.get_mut(*weapon_handle);
                        // If actor already has weapon of given kind, then just add ammo to it.
                        if weapon.get_kind() == weapon_kind {
                            found = true;
                            weapon.add_ammo(200);
                            break;
                        }
                    }
                    // Finally if actor does not have such weapon, give new one to him.
                    if !found {
                        self.give_new_weapon(engine, actor, weapon_kind);
                    }
                }
                ItemKind::Plasma | ItemKind::Ak47Ammo | ItemKind::M4Ammo => {
                    for weapon in character.weapons() {
                        let weapon = self.weapons.get_mut(*weapon);
                        let (weapon_kind, ammo) = match kind {
                            ItemKind::Plasma => (WeaponKind::PlasmaRifle, 200),
                            ItemKind::Ak47Ammo => (WeaponKind::Ak47, 200),
                            ItemKind::M4Ammo => (WeaponKind::M4, 200),
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
            self.play_sound(engine, "data/sounds/item_pickup.ogg", position, 1.0, 3.0, 2.0);
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

    fn play_sound<P: AsRef<Path>>(&self, engine: &mut GameEngine, path: P, position: Vec3, gain: f32, rolloff_factor: f32, radius: f32) {
        let mut sound_context = engine.sound_context.lock().unwrap();
        let shot_buffer = engine.resource_manager.request_sound_buffer(path, false).unwrap();
        let shot_sound = SpatialSourceBuilder::new(
            GenericSourceBuilder::new(shot_buffer)
                .with_status(Status::Playing)
                .with_play_once(true)
                .with_gain(gain)
                .build()
                .unwrap())
            .with_position(position)
            .with_radius(radius)
            .with_rolloff_factor(rolloff_factor)
            .build_source();
        sound_context.add_source(shot_sound);
    }

    fn shoot_weapon(&mut self,
                    engine: &mut GameEngine,
                    weapon_handle: Handle<Weapon>,
                    initial_velocity: Vec3,
                    time: GameTime,
                    direction: Option<Vec3>,
    ) {
        if self.weapons.contains(weapon_handle) {
            let scene = engine.scenes.get_mut(self.scene);
            let weapon = self.weapons.get_mut(weapon_handle);
            if weapon.try_shoot(scene, time) {
                let kind = weapon.definition.projectile;
                let position = weapon.get_shot_position(&scene.graph);
                let direction = direction.unwrap_or_else(|| weapon.get_shot_direction(&scene.graph))
                    .normalized()
                    .unwrap_or_else(|| Vec3::LOOK);
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
                let position = actor.character().position(&scene.physics);
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

    fn damage_actor(&mut self, engine: &GameEngine, actor: Handle<Actor>, who: Handle<Actor>, amount: f32, time: GameTime) {
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

            let who_position =
                if who.is_some() {
                    let scene = engine.scenes.get(self.scene);
                    Some(self.actors.get(who).character().position(&scene.physics))
                } else {
                    None
                };
            let actor = self.actors.get_mut(actor);
            if let Actor::Bot(bot) = actor {
                if let Some(who_position) = who_position {
                    bot.set_point_of_interest(who_position, time);
                }
            }
            let was_dead = actor.character().is_dead();
            actor.character_mut().damage(amount);
            if !was_dead && actor.character().is_dead() && who.is_some() {
                self.leader_board.add_frag(who_name)
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

    fn update_respawn(&mut self, time: GameTime) {
        // Respawn is done in deferred manner: we just gather all info needed
        // for respawn, wait some time and then re-create actor. Actor is spawned
        // by sending a message: this is needed because there are some other
        // systems that catches such messages and updates their own state.
        for respawn_entry in self.respawn_list.iter_mut() {
            match respawn_entry {
                RespawnEntry::Bot(v) => {
                    v.time_left -= time.delta;
                    if v.time_left <= 0.0 {
                        self.sender
                            .as_mut()
                            .unwrap()
                            .send(Message::SpawnBot {
                                kind: v.kind,
                                name: v.name.clone(),
                            })
                            .unwrap();
                    }
                }
                RespawnEntry::Player(v) => {
                    v.time_left -= time.delta;
                    if v.time_left <= 0.0 {
                        self.sender
                            .as_mut()
                            .unwrap()
                            .send(Message::SpawnPlayer)
                            .unwrap();
                    }
                }
            }
        }

        self.respawn_list.retain(|entry| {
            match entry {
                RespawnEntry::Bot(v) => v.time_left >= 0.0,
                RespawnEntry::Player(v) => v.time_left >= 0.0,
            }
        });
    }

    fn update_spectator_camera(&mut self, scene: &mut Scene) {
        if let Node::Camera(spectator_camera) = scene.graph.get_mut(self.spectator_camera) {
            let mut position = spectator_camera.base().get_global_position();
            position.follow(&self.target_spectator_position, 0.1);
            spectator_camera.base_mut().get_local_transform_mut().set_position(position);
        }
    }

    fn update_death_zones(&mut self, scene: &Scene) {
        for (handle, actor) in self.actors.pair_iter_mut() {
            for death_zone in self.death_zones.iter() {
                if death_zone.bounds.is_contains_point(actor.character().position(&scene.physics)) {
                    self.sender
                        .as_ref()
                        .unwrap()
                        .send(Message::RespawnActor { actor: handle })
                        .unwrap();
                }
            }
        }
    }

    pub fn update(&mut self, engine: &mut GameEngine, time: GameTime) {
        self.time += time.delta;
        self.update_respawn(time);
        let scene = engine.scenes.get_mut(self.scene);
        self.update_spectator_camera(scene);
        self.update_death_zones(scene);
        self.weapons.update(scene, &self.actors);
        self.projectiles.update(
            scene,
            &mut self.actors,
            &self.weapons,
            time,
        );
        self.items.update(scene, time);
        self.actors.update(&mut UpdateContext {
            time,
            scene,
            sound_context: engine.sound_context.clone(),
            items: &self.items,
            jump_pads: &self.jump_pads,
            navmesh: self.navmesh.as_mut(),
            weapons: &self.weapons,
        });
    }

    pub fn respawn_actor(&mut self, engine: &mut GameEngine, actor: Handle<Actor>) {
        if self.actors.contains(actor) {
            let name = self.actors.get(actor).character().name.clone();

            self.leader_board.add_death(&name);

            let entry = match self.actors.get(actor) {
                Actor::Bot(bot) => {
                    RespawnEntry::Bot(BotRespawnEntry {
                        name,
                        kind: bot.definition.kind,
                        time_left: RESPAWN_TIME,
                    })
                }
                Actor::Player(player) => {
                    // Turn on spectator camera and prepare its target position. Spectator
                    // camera will be used to render world until player is despawned.
                    let scene = engine.scenes.get_mut(self.scene);
                    let position = scene.graph.get(player.camera()).base().get_global_position();
                    if let Node::Camera(spectator_camera) = scene.graph.get_mut(self.spectator_camera) {
                        spectator_camera
                            .set_enabled(true)
                            .base_mut()
                            .get_local_transform_mut()
                            .set_position(position);
                    }
                    // Use ray casting to get target position for spectator camera, it is used to
                    // create "dropping head" effect.
                    let ray = Ray::from_two_points(&position, &(position - Vec3::new(0.0, 1000.0, 0.0))).unwrap();
                    let options = RayCastOptions {
                        ignore_bodies: true,
                        ignore_static_geometries: false,
                        sort_results: true,
                    };
                    let mut result = Vec::new();
                    if scene.physics.ray_cast(&ray, options, &mut result) {
                        if let Some(hit) = result.first() {
                            self.target_spectator_position = hit.position;
                            // Prevent see-thru-floor
                            self.target_spectator_position.y += 0.1;
                        } else {
                            self.target_spectator_position = position;
                        }
                    } else {
                        self.target_spectator_position = position;
                    }

                    RespawnEntry::Player(PlayerRespawnEntry {
                        time_left: RESPAWN_TIME
                    })
                }
            };

            self.remove_actor(engine, actor);

            self.respawn_list.push(entry);
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
            Message::ShootWeapon { weapon, initial_velocity, direction } => {
                self.shoot_weapon(engine, *weapon, *initial_velocity, time, direction.clone())
            }
            Message::CreateProjectile { kind, position, direction, initial_velocity, owner } => {
                self.create_projectile(engine, *kind, *position, *direction, *initial_velocity, *owner)
            }
            Message::PlaySound { path, position, gain, rolloff_factor, radius } => {
                self.play_sound(engine, path, *position, *gain, *rolloff_factor, *radius)
            }
            Message::ShowWeapon { weapon, state } => {
                self.show_weapon(engine, *weapon, *state)
            }
            Message::SpawnBot { kind, name } => {
                self.spawn_bot(engine, *kind, Some(name.clone()));
            }
            Message::DamageActor { actor, who, amount } => {
                self.damage_actor(engine, *actor, *who, *amount, time);
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