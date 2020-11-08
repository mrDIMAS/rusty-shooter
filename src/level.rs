use crate::{
    actor::{Actor, ActorContainer},
    bot::{Bot, BotKind},
    control_scheme::ControlScheme,
    effects,
    item::{Item, ItemContainer, ItemKind},
    jump_pad::{JumpPad, JumpPadContainer},
    leader_board::LeaderBoard,
    message::Message,
    player::Player,
    projectile::{Projectile, ProjectileContainer, ProjectileKind},
    weapon::{Weapon, WeaponContainer, WeaponKind},
    GameEngine, GameTime, MatchOptions,
};
use rand::Rng;
use rg3d::core::algebra::Matrix3;
use rg3d::physics::geometry::{ContactEvent, InteractionGroups, ProximityEvent};
use rg3d::scene::physics::RayCastOptions;
use rg3d::{
    core::{
        algebra::Vector3,
        color::Color,
        math::{aabb::AxisAlignedBoundingBox, ray::Ray, PositionProvider},
        pool::Handle,
        visitor::{Visit, VisitResult, Visitor},
    },
    event::Event,
    scene,
    scene::{base::BaseBuilder, camera::CameraBuilder, node::Node, Scene},
    sound::context::Context,
    utils::{self, navmesh::Navmesh},
};
use std::path::PathBuf;
use std::{
    cell::RefCell,
    path::Path,
    rc::Rc,
    sync::{mpsc::Sender, Arc, Mutex},
};

use crate::rg3d::core::math::Vector3Ext;
use rg3d::physics::pipeline::{ChannelEventCollector, EventHandler};
use std::sync::mpsc::{channel, Receiver};

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
    target_spectator_position: Vector3<f32>,
    proximity_events_receiver: Option<crossbeam::channel::Receiver<ProximityEvent>>,
    contact_events_receiver: Option<crossbeam::channel::Receiver<ContactEvent>>,
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
            proximity_events_receiver: None,
            contact_events_receiver: None,
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
        self.target_spectator_position
            .visit("TargetSpectatorPosition", visitor)?;

        visitor.leave_region()
    }
}

pub struct DeathZone {
    bounds: AxisAlignedBoundingBox,
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
            bounds: Default::default(),
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
    time_left: f32,
}

impl Default for PlayerRespawnEntry {
    fn default() -> Self {
        Self { time_left: 0.0 }
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
            _ => Err(format!("Invalid RespawnEntry type {}", id)),
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
    pub async fn new(
        engine: &mut GameEngine,
        control_scheme: Rc<RefCell<ControlScheme>>,
        sender: Sender<Message>,
        options: MatchOptions,
    ) -> Level {
        let mut scene = Scene::new();

        let (proximity_events_sender, proximity_events_receiver) = crossbeam::channel::unbounded();
        let (contact_events_sender, contact_events_receiver) = crossbeam::channel::unbounded();

        scene.physics.event_handler = Box::new(ChannelEventCollector::new(
            proximity_events_sender.clone(),
            contact_events_sender.clone(),
        ));

        // Spectator camera is used when there is no player on level.
        // This includes situation when player is dead - all dead actors are removed
        // from level.
        let spectator_camera = scene.graph.add_node(Node::Camera(
            CameraBuilder::new(BaseBuilder::new())
                .enabled(false)
                .build(),
        ));

        let map_model = engine
            .resource_manager
            .request_model(Path::new("data/models/dm6.fbx"))
            .await
            .unwrap();

        // Instantiate map
        let map_root = map_model.instantiate_geometry(&mut scene);
        // Create collision geometry
        let polygon_handle = scene.graph.find_by_name(map_root, "Polygon");
        if polygon_handle.is_some() {
            let collider = scene
                .physics
                .mesh_to_trimesh(scene.graph[polygon_handle].as_mesh());
            scene.physics_binder.bind(polygon_handle, collider);
        } else {
            println!("Unable to find Polygon node to build collision shape for level!");
        }

        let mut level = Level {
            scene: engine.scenes.add(scene),
            sender: Some(sender),
            control_scheme: Some(control_scheme),
            map_root,
            options,
            spectator_camera,
            contact_events_receiver: Some(contact_events_receiver),
            proximity_events_receiver: Some(proximity_events_receiver),
            ..Default::default()
        };

        level.build_navmesh(engine);
        level.analyze(engine).await;
        level.spawn_player(engine).await;
        level
            .spawn_bot(engine, BotKind::Maw, Some("Maw".to_owned()))
            .await;
        level
            .spawn_bot(engine, BotKind::Mutant, Some("Mutant".to_owned()))
            .await;
        level
            .spawn_bot(engine, BotKind::Parasite, Some("Parasite".to_owned()))
            .await;

        level
    }

    pub fn build_navmesh(&mut self, engine: &mut GameEngine) {
        if self.navmesh.is_none() {
            let scene = &mut engine.scenes[self.scene];
            let navmesh_handle = scene.graph.find_by_name(self.map_root, "Navmesh");
            if navmesh_handle.is_some() {
                let navmesh_node = &mut scene.graph[navmesh_handle];
                navmesh_node.set_visibility(false);
                self.navmesh = Some(Navmesh::from_mesh(navmesh_node.as_mesh()));
            } else {
                println!("Unable to find Navmesh node to build navmesh!")
            }
        }
    }

    pub async fn analyze(&mut self, engine: &mut GameEngine) {
        let mut items = Vec::new();
        let mut spawn_points = Vec::new();
        let mut death_zones = Vec::new();
        let scene = &mut engine.scenes[self.scene];
        for (handle, node) in scene.graph.pair_iter() {
            let position = node.global_position();
            let name = node.name();
            if name.starts_with("JumpPad") {
                let begin = scene
                    .graph
                    .find_by_name_from_root(format!("{}_Begin", name).as_str());
                let end = scene
                    .graph
                    .find_by_name_from_root(format!("{}_End", name).as_str());
                if begin.is_some() && end.is_some() {
                    let begin = scene.graph[begin].global_position();
                    let end = scene.graph[end].global_position();
                    let d = end - begin;
                    let len = d.norm();
                    let force = d.try_normalize(std::f32::EPSILON);
                    let force = force.unwrap_or(Vector3::y()).scale(len * 3.0);
                    let shape = scene.physics.mesh_to_trimesh(node.as_mesh());
                    scene.physics_binder.bind(handle, shape);
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
                spawn_points.push(node.global_position())
            } else if name.starts_with("DeathZone") {
                if let Node::Mesh(_) = node {
                    death_zones.push(handle);
                }
            }
        }
        for (kind, position) in items {
            self.items.add(
                Item::new(
                    kind,
                    position,
                    scene,
                    engine.resource_manager.clone(),
                    self.sender.as_ref().unwrap().clone(),
                )
                .await,
            );
        }
        for handle in death_zones {
            let node = &mut scene.graph[handle];
            node.set_visibility(false);
            self.death_zones.push(DeathZone {
                bounds: node.as_mesh().world_bounding_box(),
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

    pub fn actors(&self) -> &ActorContainer {
        &self.actors
    }

    pub fn actors_mut(&mut self) -> &mut ActorContainer {
        &mut self.actors
    }

    pub fn weapons(&self) -> &WeaponContainer {
        &self.weapons
    }

    fn pick(&self, engine: &mut GameEngine, from: Vector3<f32>, to: Vector3<f32>) -> Vector3<f32> {
        let scene = &mut engine.scenes[self.scene];
        if let Some(ray) = Ray::from_two_points(&from, &to) {
            let options = RayCastOptions {
                ray,
                max_len: std::f32::MAX,
                groups: InteractionGroups::all(),
                sort_results: true,
            };
            let mut query_buffer = Vec::default();
            scene.physics.cast_ray(options, &mut query_buffer);
            if let Some(pt) = query_buffer.first() {
                pt.position.coords
            } else {
                from
            }
        } else {
            from
        }
    }

    fn remove_weapon(&mut self, engine: &mut GameEngine, weapon: Handle<Weapon>) {
        for projectile in self.projectiles.iter_mut() {
            if projectile.owner == weapon {
                // Reset owner because handle to weapon will be invalid after weapon freed.
                projectile.owner = Handle::NONE;
            }
        }
        self.weapons[weapon].clean_up(&mut engine.scenes[self.scene]);
        self.weapons.free(weapon);
    }

    async fn give_new_weapon(
        &mut self,
        engine: &mut GameEngine,
        actor: Handle<Actor>,
        kind: WeaponKind,
    ) {
        if self.actors.contains(actor) {
            let scene = &mut engine.scenes[self.scene];
            let mut weapon = Weapon::new(
                kind,
                engine.resource_manager.clone(),
                scene,
                self.sender.as_ref().unwrap().clone(),
            )
            .await;
            weapon.set_owner(actor);
            let weapon_model = weapon.get_model();
            let actor = self.actors.get_mut(actor);
            let weapon_handle = self.weapons.add(weapon);
            actor.add_weapon(weapon_handle);
            scene.graph.link_nodes(weapon_model, actor.weapon_pivot());

            self.sender
                .as_ref()
                .unwrap()
                .send(Message::AddNotification {
                    text: format!("Actor picked up weapon {:?}", kind),
                })
                .unwrap();
        }
    }

    async fn add_bot(
        &mut self,
        engine: &mut GameEngine,
        kind: BotKind,
        position: Vector3<f32>,
        name: Option<String>,
    ) -> Handle<Actor> {
        let scene = &mut engine.scenes[self.scene];
        let bot = Bot::new(
            kind,
            engine.resource_manager.clone(),
            scene,
            position,
            self.sender.as_ref().unwrap().clone(),
        )
        .await;
        let name = name.unwrap_or_else(|| format!("Bot {:?} {}", kind, self.actors.count()));
        self.leader_board.get_or_add_actor(&name);
        let bot = self.actors.add(Actor::Bot(bot));
        self.give_new_weapon(engine, bot, WeaponKind::Ak47).await;
        bot
    }

    async fn remove_actor(&mut self, engine: &mut GameEngine, actor: Handle<Actor>) {
        if self.actors.contains(actor) {
            let scene = &mut engine.scenes[self.scene];
            let character = self.actors.get(actor);

            // Make sure to remove weapons and drop appropriate items (items will be temporary).
            let drop_position = character.position(&scene.physics);
            let weapons = character
                .weapons()
                .iter()
                .copied()
                .collect::<Vec<Handle<Weapon>>>();
            for weapon in weapons {
                let item_kind = match self.weapons[weapon].get_kind() {
                    WeaponKind::M4 => ItemKind::M4,
                    WeaponKind::Ak47 => ItemKind::Ak47,
                    WeaponKind::PlasmaRifle => ItemKind::PlasmaGun,
                    WeaponKind::RocketLauncher => ItemKind::RocketLauncher,
                };
                self.spawn_item(engine, item_kind, drop_position, true, Some(20.0))
                    .await;
                self.remove_weapon(engine, weapon);
            }

            let scene = &mut engine.scenes[self.scene];
            self.actors.get_mut(actor).clean_up(scene);
            self.actors.free(actor);

            if self.player == actor {
                self.player = Handle::NONE;
            }
        }
    }

    async fn spawn_player(&mut self, engine: &mut GameEngine) -> Handle<Actor> {
        let index = self.find_suitable_spawn_point(engine);
        let spawn_position = self
            .spawn_points
            .get(index)
            .map_or(Vector3::default(), |pt| {
                pt.position + Vector3::new(0.0, 1.5, 0.0)
            });
        let scene = &mut engine.scenes[self.scene];
        if let Node::Camera(spectator_camera) = &mut scene.graph[self.spectator_camera] {
            spectator_camera.set_enabled(false);
        }
        let mut player = Player::new(scene, self.sender.as_ref().unwrap().clone());
        if let Some(control_scheme) = self.control_scheme.as_ref() {
            player.set_control_scheme(control_scheme.clone());
        }
        self.player = self.actors.add(Actor::Player(player));
        self.actors
            .get_mut(self.player)
            .set_position(&mut scene.physics, spawn_position);

        self.give_new_weapon(engine, self.player, WeaponKind::M4)
            .await;
        self.give_new_weapon(engine, self.player, WeaponKind::Ak47)
            .await;
        self.give_new_weapon(engine, self.player, WeaponKind::PlasmaRifle)
            .await;
        self.give_new_weapon(engine, self.player, WeaponKind::RocketLauncher)
            .await;

        self.player
    }

    async fn give_item(&mut self, engine: &mut GameEngine, actor: Handle<Actor>, kind: ItemKind) {
        if self.actors.contains(actor) {
            let character = self.actors.get_mut(actor);
            match kind {
                ItemKind::Medkit => character.heal(20.0),
                ItemKind::Ak47 | ItemKind::PlasmaGun | ItemKind::M4 | ItemKind::RocketLauncher => {
                    let weapon_kind = match kind {
                        ItemKind::Ak47 => WeaponKind::Ak47,
                        ItemKind::PlasmaGun => WeaponKind::PlasmaRifle,
                        ItemKind::M4 => WeaponKind::M4,
                        ItemKind::RocketLauncher => WeaponKind::RocketLauncher,
                        _ => unreachable!(),
                    };

                    let mut found = false;
                    for weapon_handle in character.weapons() {
                        let weapon = &mut self.weapons[*weapon_handle];
                        // If actor already has weapon of given kind, then just add ammo to it.
                        if weapon.get_kind() == weapon_kind {
                            found = true;
                            weapon.add_ammo(200);
                            break;
                        }
                    }
                    // Finally if actor does not have such weapon, give new one to him.
                    if !found {
                        self.give_new_weapon(engine, actor, weapon_kind).await;
                    }
                }
                ItemKind::Plasma | ItemKind::Ak47Ammo | ItemKind::M4Ammo => {
                    for weapon in character.weapons() {
                        let weapon = &mut self.weapons[*weapon];
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

    async fn pickup_item(
        &mut self,
        engine: &mut GameEngine,
        actor: Handle<Actor>,
        item: Handle<Item>,
    ) {
        if self.actors.contains(actor) && self.items.contains(item) {
            let item = self.items.get_mut(item);

            self.sender
                .as_ref()
                .unwrap()
                .send(Message::AddNotification {
                    text: format!("Actor picked up item {:?}", item.get_kind()),
                })
                .unwrap();

            let scene = &mut engine.scenes[self.scene];
            let position = item.position(&scene.graph);
            item.pick_up();
            let kind = item.get_kind();
            self.sender
                .as_ref()
                .unwrap()
                .send(Message::PlaySound {
                    path: PathBuf::from("data/sounds/item_pickup.ogg"),
                    position,
                    gain: 1.0,
                    rolloff_factor: 3.0,
                    radius: 2.0,
                })
                .unwrap();
            self.give_item(engine, actor, kind).await;
        }
    }

    async fn create_projectile(
        &mut self,
        engine: &mut GameEngine,
        kind: ProjectileKind,
        position: Vector3<f32>,
        direction: Vector3<f32>,
        initial_velocity: Vector3<f32>,
        owner: Handle<Weapon>,
        basis: Matrix3<f32>,
    ) {
        let scene = &mut engine.scenes[self.scene];
        let projectile = Projectile::new(
            kind,
            engine.resource_manager.clone(),
            scene,
            direction,
            position,
            owner,
            initial_velocity,
            self.sender.as_ref().unwrap().clone(),
            basis,
        )
        .await;
        self.projectiles.add(projectile);
    }

    async fn shoot_weapon(
        &mut self,
        engine: &mut GameEngine,
        weapon_handle: Handle<Weapon>,
        initial_velocity: Vector3<f32>,
        time: GameTime,
        direction: Option<Vector3<f32>>,
    ) {
        if self.weapons.contains(weapon_handle) {
            let scene = &mut engine.scenes[self.scene];
            let weapon = &mut self.weapons[weapon_handle];
            if weapon.try_shoot(scene, time) {
                let kind = weapon.definition.projectile;
                let position = weapon.get_shot_position(&scene.graph);
                let direction = direction
                    .unwrap_or_else(|| weapon.get_shot_direction(&scene.graph))
                    .try_normalize(std::f32::EPSILON)
                    .unwrap_or_else(|| Vector3::z());
                let basis = weapon.world_basis(&scene.graph);
                self.create_projectile(
                    engine,
                    kind,
                    position,
                    direction,
                    initial_velocity,
                    weapon_handle,
                    basis,
                )
                .await;
            }
        }
    }

    fn show_weapon(&mut self, engine: &mut GameEngine, weapon_handle: Handle<Weapon>, state: bool) {
        self.weapons[weapon_handle].set_visibility(state, &mut engine.scenes[self.scene].graph)
    }

    fn find_suitable_spawn_point(&self, engine: &mut GameEngine) -> usize {
        // Find spawn point with least amount of enemies nearby.
        let scene = &mut engine.scenes[self.scene];
        let mut index = rand::thread_rng().gen_range(0, self.spawn_points.len());
        let mut max_distance = -std::f32::MAX;
        for (i, pt) in self.spawn_points.iter().enumerate() {
            let mut sum_distance = 0.0;
            for actor in self.actors.iter() {
                let position = actor.position(&scene.physics);
                sum_distance += pt.position.metric_distance(&position);
            }
            if sum_distance > max_distance {
                max_distance = sum_distance;
                index = i;
            }
        }
        index
    }

    async fn spawn_bot(
        &mut self,
        engine: &mut GameEngine,
        kind: BotKind,
        name: Option<String>,
    ) -> Handle<Actor> {
        let index = self.find_suitable_spawn_point(engine);
        let spawn_position = self
            .spawn_points
            .get(index)
            .map_or(Vector3::default(), |pt| pt.position);

        let bot = self.add_bot(engine, kind, spawn_position, name).await;

        self.sender
            .as_ref()
            .unwrap()
            .send(Message::AddNotification {
                text: format!("Bot {} spawned!", self.actors.get(bot).name),
            })
            .unwrap();

        bot
    }

    fn damage_actor(
        &mut self,
        engine: &GameEngine,
        actor: Handle<Actor>,
        who: Handle<Actor>,
        amount: f32,
        time: GameTime,
    ) {
        if self.actors.contains(actor)
            && (who.is_none() || who.is_some() && self.actors.contains(who))
        {
            let mut who_name = Default::default();
            let message = if who.is_some() {
                who_name = self.actors.get(who).name.clone();
                format!(
                    "{} dealt {} damage to {}!",
                    who_name,
                    amount,
                    self.actors.get(actor).name
                )
            } else {
                format!("{} took {} damage!", self.actors.get(actor).name, amount)
            };

            self.sender
                .as_ref()
                .unwrap()
                .send(Message::AddNotification { text: message })
                .unwrap();

            let who_position = if who.is_some() {
                let scene = &engine.scenes[self.scene];
                Some(self.actors.get(who).position(&scene.physics))
            } else {
                None
            };
            let actor = self.actors.get_mut(actor);
            if let Actor::Bot(bot) = actor {
                if let Some(who_position) = who_position {
                    bot.set_point_of_interest(who_position, time);
                }
            }
            let was_dead = actor.is_dead();
            actor.damage(amount);
            if !was_dead && actor.is_dead() && who.is_some() {
                self.leader_board.add_frag(who_name)
            }
        }
    }

    async fn spawn_item(
        &mut self,
        engine: &mut GameEngine,
        kind: ItemKind,
        position: Vector3<f32>,
        adjust_height: bool,
        lifetime: Option<f32>,
    ) {
        let position = if adjust_height {
            self.pick(engine, position, position - Vector3::new(0.0, 1000.0, 0.0))
        } else {
            position
        };
        let scene = &mut engine.scenes[self.scene];
        let mut item = Item::new(
            kind,
            position,
            scene,
            engine.resource_manager.clone(),
            self.sender.as_ref().unwrap().clone(),
        )
        .await;
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

        self.respawn_list.retain(|entry| match entry {
            RespawnEntry::Bot(v) => v.time_left >= 0.0,
            RespawnEntry::Player(v) => v.time_left >= 0.0,
        });
    }

    fn update_spectator_camera(&mut self, scene: &mut Scene) {
        if let Node::Camera(spectator_camera) = &mut scene.graph[self.spectator_camera] {
            let mut position = spectator_camera.global_position();
            position.follow(&self.target_spectator_position, 0.1);
            spectator_camera
                .local_transform_mut()
                .set_position(position);
        }
    }

    fn update_death_zones(&mut self, scene: &Scene) {
        for (handle, actor) in self.actors.pair_iter_mut() {
            for death_zone in self.death_zones.iter() {
                if death_zone
                    .bounds
                    .is_contains_point(actor.position(&scene.physics))
                {
                    self.sender
                        .as_ref()
                        .unwrap()
                        .send(Message::RespawnActor { actor: handle })
                        .unwrap();
                }
            }
        }
    }

    fn update_game_ending(&self) {
        if self.leader_board.is_match_over(&self.options) {
            self.sender
                .as_ref()
                .unwrap()
                .send(Message::EndMatch)
                .unwrap();
        }
    }

    pub fn update(&mut self, engine: &mut GameEngine, time: GameTime) {
        self.time += time.delta;
        self.update_respawn(time);
        let scene = &mut engine.scenes[self.scene];
        self.update_spectator_camera(scene);
        self.update_death_zones(scene);
        self.weapons.update(scene, &self.actors);
        self.projectiles
            .update(scene, &self.actors, &self.weapons, time);
        self.items.update(scene, time);
        let mut ctx = UpdateContext {
            time,
            scene,
            sound_context: engine.sound_context.clone(),
            items: &self.items,
            jump_pads: &self.jump_pads,
            navmesh: self.navmesh.as_mut(),
            weapons: &self.weapons,
        };
        self.actors.update(&mut ctx);
        while let Ok(contact_event) = self.contact_events_receiver.as_ref().unwrap().try_recv() {
            self.actors.handle_event(&contact_event, &mut ctx);
        }
        while let Ok(_proximity_event) = self.proximity_events_receiver.as_ref().unwrap().try_recv()
        {
            // Just drain proximity events. We don't need them.
        }
        self.update_game_ending();
    }

    pub async fn respawn_actor(&mut self, engine: &mut GameEngine, actor: Handle<Actor>) {
        if self.actors.contains(actor) {
            let name = self.actors.get(actor).name.clone();

            self.leader_board.add_death(&name);

            let entry = match self.actors.get(actor) {
                Actor::Bot(bot) => RespawnEntry::Bot(BotRespawnEntry {
                    name,
                    kind: bot.definition.kind,
                    time_left: RESPAWN_TIME,
                }),
                Actor::Player(player) => {
                    // Turn on spectator camera and prepare its target position. Spectator
                    // camera will be used to render world until player is despawned.
                    let scene = &mut engine.scenes[self.scene];
                    let position = scene.graph[player.camera()].global_position();
                    if let Node::Camera(spectator_camera) = &mut scene.graph[self.spectator_camera]
                    {
                        spectator_camera
                            .set_enabled(true)
                            .local_transform_mut()
                            .set_position(position);
                    }
                    // Use ray casting to get target position for spectator camera, it is used to
                    // create "dropping head" effect.
                    let ray = Ray::from_two_points(
                        &position,
                        &(position - Vector3::new(0.0, 1000.0, 0.0)),
                    )
                    .unwrap();
                    let options = RayCastOptions {
                        ray,
                        max_len: std::f32::MAX,
                        groups: InteractionGroups::all(),
                        sort_results: true,
                    };

                    let mut query_buffer = Vec::default();
                    scene.physics.cast_ray(options, &mut query_buffer);
                    if let Some(hit) = query_buffer.first() {
                        self.target_spectator_position = hit.position.coords;
                        // Prevent see-thru-floor
                        self.target_spectator_position.y += 0.1;
                    } else {
                        self.target_spectator_position = position;
                    }

                    RespawnEntry::Player(PlayerRespawnEntry {
                        time_left: RESPAWN_TIME,
                    })
                }
            };

            self.remove_actor(engine, actor).await;

            self.respawn_list.push(entry);
        }
    }

    pub async fn handle_message(
        &mut self,
        engine: &mut GameEngine,
        message: &Message,
        time: GameTime,
    ) {
        match message {
            &Message::GiveNewWeapon { actor, kind } => {
                self.give_new_weapon(engine, actor, kind).await;
            }
            Message::AddBot {
                kind,
                position,
                name,
            } => {
                self.add_bot(engine, *kind, *position, name.clone()).await;
            }
            &Message::RemoveActor { actor } => self.remove_actor(engine, actor).await,
            &Message::GiveItem { actor, kind } => {
                self.give_item(engine, actor, kind).await;
            }
            &Message::PickUpItem { actor, item } => {
                self.pickup_item(engine, actor, item).await;
            }
            &Message::ShootWeapon {
                weapon,
                initial_velocity,
                direction,
            } => {
                self.shoot_weapon(engine, weapon, initial_velocity, time, direction)
                    .await
            }
            &Message::CreateProjectile {
                kind,
                position,
                direction,
                initial_velocity,
                owner,
                basis,
            } => {
                self.create_projectile(
                    engine,
                    kind,
                    position,
                    direction,
                    initial_velocity,
                    owner,
                    basis,
                )
                .await
            }
            &Message::ShowWeapon { weapon, state } => self.show_weapon(engine, weapon, state),
            Message::SpawnBot { kind, name } => {
                self.spawn_bot(engine, *kind, Some(name.clone())).await;
            }
            &Message::DamageActor { actor, who, amount } => {
                self.damage_actor(engine, actor, who, amount, time);
            }
            &Message::CreateEffect { kind, position } => effects::create(
                kind,
                &mut engine.scenes[self.scene].graph,
                engine.resource_manager.clone(),
                position,
            ),
            Message::SpawnPlayer => {
                self.spawn_player(engine).await;
            }
            &Message::SpawnItem {
                kind,
                position,
                adjust_height,
                lifetime,
            } => {
                self.spawn_item(engine, kind, position, adjust_height, lifetime)
                    .await
            }
            &Message::RespawnActor { actor } => self.respawn_actor(engine, actor).await,
            _ => (),
        }
    }

    pub fn set_message_sender(&mut self, sender: Sender<Message>) {
        self.sender = Some(sender.clone());

        // Attach new sender to all event sources.
        for actor in self.actors.iter_mut() {
            actor.sender = Some(sender.clone());
        }
        for weapon in self.weapons.iter_mut() {
            weapon.sender = Some(sender.clone());
        }
        for projectile in self.projectiles.iter_mut() {
            projectile.sender = Some(sender.clone());
        }
    }

    pub fn debug_draw(&self, engine: &mut GameEngine) {
        let drawing_context = &mut engine.scenes[self.scene].drawing_context;

        drawing_context.clear_lines();

        if let Some(navmesh) = self.navmesh.as_ref() {
            for pt in navmesh.vertices() {
                for neighbour in pt.neighbours() {
                    drawing_context.add_line(scene::Line {
                        begin: pt.position(),
                        end: navmesh.vertices()[*neighbour].position(),
                        color: Default::default(),
                    });
                }
            }

            for actor in self.actors.iter() {
                if let Actor::Bot(bot) = actor {
                    bot.debug_draw(drawing_context);
                }
            }
        }

        for death_zone in self.death_zones.iter() {
            drawing_context.draw_aabb(&death_zone.bounds, Color::opaque(0, 0, 200));
        }
    }
}

pub struct SpawnPoint {
    position: Vector3<f32>,
}

impl Default for SpawnPoint {
    fn default() -> Self {
        Self {
            position: Default::default(),
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
