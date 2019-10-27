use rg3d_core::{
    pool::{Handle, Pool, PoolIterator},
    visitor::{Visit, Visitor, VisitError, VisitResult},
    math::vec3::Vec3
};
use rg3d::{
    engine::resource_manager::ResourceManager,
    resource::model::Model,
    scene::{
        base::{
            BaseBuilder,
            AsBase
        },
        Scene,
        SceneInterfaceMut,
        node::Node,
        transform::TransformBuilder
    }
};
use std::path::Path;
use crate::GameTime;
use rg3d::scene::graph::Graph;

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum ItemKind {
    Medkit = 0,
}

impl ItemKind {
    fn from_id(id: u32) -> Result<ItemKind, String> {
        match id {
            0 => Ok(ItemKind::Medkit),
            _ => Err(format!("Unknown item kind {}", id))
        }
    }

    fn id(&self) -> u32 {
        match self {
            ItemKind::Medkit => 0,
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
}

impl Default for Item {
    fn default() -> Self {
        Self {
            kind: ItemKind::Medkit,
            pivot: Default::default(),
            model: Default::default(),
            offset: Default::default(),
            dest_offset: Default::default(),
            offset_factor: 0.0
        }
    }
}

impl Item {
    pub fn new(
        kind: ItemKind,
        position: Vec3,
        scene: &mut Scene,
        resource_manager: &mut ResourceManager
    ) -> Self {
        let model = match kind {
            ItemKind::Medkit => {
                let model = resource_manager.request_model(Path::new("data/models/medkit.fbx")).unwrap();
                Model::instantiate_geometry(model, scene)
            },
        };

        let SceneInterfaceMut { graph, .. } = scene.interface_mut();

        let pivot = graph.add_node(Node::Base(BaseBuilder::new()
            .with_local_transform(TransformBuilder::new()
                .with_local_position(position)
                .build())
            .build()));

        graph.link_nodes(model, pivot);

        Self {
            pivot,
            kind,
            model,
            .. Default::default()
        }
    }
}

impl Visit for Item {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.model.visit("Model", visitor)?;
        self.pivot.visit("Pivot", visitor)?;
        self.offset.visit("Offset", visitor)?;
        self.offset_factor.visit("OffsetFactor", visitor)?;
        self.dest_offset.visit("DestOffset", visitor)?;

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

    pub fn iter(&self) -> PoolIterator<Item> {
        self.pool.iter()
    }

    pub fn update(&mut self, scene: &mut Scene, time: GameTime) {
        let SceneInterfaceMut { graph, .. } = scene.interface_mut();

        for item in self.pool.iter_mut() {
            item.offset_factor += 1.2 * time.delta;

            item.dest_offset = Vec3::new(0.0, 0.085 * item.offset_factor.sin(), 0.0);
            item.offset.follow(&item.dest_offset, 0.2);

            graph.get_mut(item.model).base_mut().get_local_transform_mut().set_position(item.offset);
        }
    }
}