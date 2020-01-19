use rg3d::{
    core::{
        pool::{
            Handle,
            Pool,
            PoolPairIterator
        },
        visitor::{Visit, Visitor, VisitResult},
        math::vec3::Vec3,
    },
    engine::resource_manager::ResourceManager,
    scene::{
        base::{
            BaseBuilder,
            AsBase,
        },
        Scene,
        SceneInterfaceMut,
        node::Node,
        transform::TransformBuilder,
        graph::Graph,
    }
};
use std::path::Path;
use crate::{GameTime, effects};

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum ItemKind {
    Medkit = 0,
    Plasma = 1,
    Ak47Ammo762 = 2,
    M4Ammo556 = 3,
}

impl ItemKind {
    fn from_id(id: u32) -> Result<ItemKind, String> {
        match id {
            0 => Ok(ItemKind::Medkit),
            1 => Ok(ItemKind::Plasma),
            2 => Ok(ItemKind::Ak47Ammo762),
            3 => Ok(ItemKind::M4Ammo556),
            _ => Err(format!("Unknown item kind {}", id))
        }
    }

    fn id(&self) -> u32 {
        match self {
            ItemKind::Medkit => 0,
            ItemKind::Plasma => 1,
            ItemKind::Ak47Ammo762 => 2,
            ItemKind::M4Ammo556 => 3,
        }
    }
}

pub struct Item {
    kind: ItemKind,
    pivot: Handle<Node>,
    model: Handle<Node>,
    offset: Vec3,
    dest_offset: Vec3,
    offset_factor: f32,
    reactivation_timer: f32,
    active: bool,
    definition: &'static ItemDefinition,
}

impl Default for Item {
    fn default() -> Self {
        Self {
            kind: ItemKind::Medkit,
            pivot: Default::default(),
            model: Default::default(),
            offset: Default::default(),
            dest_offset: Default::default(),
            offset_factor: 0.0,
            reactivation_timer: 0.0,
            active: true,
            definition: Self::get_definition(ItemKind::Medkit)
        }
    }
}

pub struct ItemDefinition {
    model: &'static str,
    scale: f32,
    reactivation_interval: f32,
}

impl Item {
    pub fn get_definition(kind: ItemKind) -> &'static ItemDefinition {
        match kind {
            ItemKind::Medkit => {
                static DEFINITION: ItemDefinition = ItemDefinition {
                    model: "data/models/medkit.fbx",
                    scale: 1.0,
                    reactivation_interval: 20.0,
                };
                &DEFINITION
            },
            ItemKind::Plasma => {
                static DEFINITION: ItemDefinition = ItemDefinition {
                    model: "data/models/yellow_box.FBX",
                    scale: 0.25,
                    reactivation_interval: 15.0,
                };
                &DEFINITION
            },
            ItemKind::Ak47Ammo762 => {
                static DEFINITION: ItemDefinition = ItemDefinition {
                    model: "data/models/box_medium.FBX",
                    scale: 0.30,
                    reactivation_interval: 14.0,
                };
                &DEFINITION
            },
            ItemKind::M4Ammo556 => {
                static DEFINITION: ItemDefinition = ItemDefinition {
                    model: "data/models/box_small.FBX",
                    scale: 0.30,
                    reactivation_interval: 13.0,
                };
                &DEFINITION
            },
        }
    }

    pub fn new(
        kind: ItemKind,
        position: Vec3,
        scene: &mut Scene,
        resource_manager: &mut ResourceManager,
    ) -> Self {
        let definition = Self::get_definition(kind);

        let model = resource_manager.request_model(Path::new(definition.model))
            .unwrap()
            .lock()
            .unwrap()
            .instantiate_geometry( scene);

        let SceneInterfaceMut { graph, .. } = scene.interface_mut();

        let pivot = graph.add_node(Node::Base(BaseBuilder::new()
            .with_local_transform(TransformBuilder::new()
                .with_local_position(position)
                .with_local_scale(Vec3::new(definition.scale, definition.scale, definition.scale))
                .build())
            .build()));

        graph.link_nodes(model, pivot);

        Self {
            pivot,
            kind,
            model,
            ..Default::default()
        }
    }

    pub fn get_pivot(&self) -> Handle<Node> {
        self.pivot
    }

    pub fn position(&self, graph: &Graph) -> Vec3 {
        graph.get(self.pivot).base().get_global_position()
    }

    pub fn update(&mut self,
                  graph: &mut Graph,
                  resource_manager: &mut ResourceManager,
                  time: GameTime
    ) {
        self.offset_factor += 1.2 * time.delta;

        self.dest_offset = Vec3::new(0.0, 0.085 * self.offset_factor.sin(), 0.0);
        self.offset.follow(&self.dest_offset, 0.2);

        let position = graph.get(self.pivot).base().get_global_position();

        let model = graph.get_mut(self.model).base_mut();
        model.get_local_transform_mut().set_position(self.offset);
        model.set_visibility(!self.is_picked_up());

        if !self.active {
            self.reactivation_timer -= time.delta;
            if self.reactivation_timer <= 0.0 {
                self.active = true;
                effects::create_item_appear(graph, resource_manager, position);
            }
        }
    }

    pub fn get_kind(&self) -> ItemKind {
        self.kind
    }

    pub fn pick_up(&mut self) {
        self.reactivation_timer = self.definition.reactivation_interval;
        self.active = false;
    }

    pub fn is_picked_up(&self) -> bool {
        !self.active
    }
}

impl Visit for Item {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        let mut kind = self.kind.id();
        kind.visit("Kind", visitor)?;
        if visitor.is_reading() {
            self.kind = ItemKind::from_id(kind)?;
        }

        self.definition = Self::get_definition(self.kind);
        self.model.visit("Model", visitor)?;
        self.pivot.visit("Pivot", visitor)?;
        self.offset.visit("Offset", visitor)?;
        self.offset_factor.visit("OffsetFactor", visitor)?;
        self.dest_offset.visit("DestOffset", visitor)?;
        self.reactivation_timer.visit("ReactivationTimer", visitor)?;
        self.active.visit("Active", visitor)?;

        visitor.leave_region()
    }
}

pub struct ItemContainer {
    pool: Pool<Item>
}

impl Default for ItemContainer {
    fn default() -> Self {
        Self::new()
    }
}

impl Visit for ItemContainer {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.pool.visit("Pool", visitor)?;

        visitor.leave_region()
    }
}

impl ItemContainer {
    pub fn new() -> Self {
        Self {
            pool: Pool::new()
        }
    }

    pub fn add(&mut self, item: Item) -> Handle<Item> {
        self.pool.spawn(item)
    }

    pub fn get_mut(&mut self, item: Handle<Item>) -> &mut Item {
        self.pool.borrow_mut(item)
    }

    pub fn pair_iter(&self) -> PoolPairIterator<Item> {
        self.pool.pair_iter()
    }

    pub fn update(&mut self,
                  scene: &mut Scene,
                  resource_manager: &mut ResourceManager,
                  time: GameTime
    ) {
        let SceneInterfaceMut { graph, .. } = scene.interface_mut();

        for item in self.pool.iter_mut() {
            item.update(graph, resource_manager, time);
        }
    }
}