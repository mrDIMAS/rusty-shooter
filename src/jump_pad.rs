use fyrox::core::{
    algebra::Vector3,
    pool::{Handle, Pool},
    visitor::{Visit, VisitResult, Visitor},
};
use fyrox::scene::node::Node;

pub struct JumpPad {
    velocity: Vector3<f32>,
    collider: Handle<Node>,
}

impl JumpPad {
    pub fn new(collider: Handle<Node>, force: Vector3<f32>) -> JumpPad {
        Self {
            velocity: force,
            collider,
        }
    }

    pub fn collider(&self) -> Handle<Node> {
        self.collider
    }

    pub fn velocity(&self) -> Vector3<f32> {
        self.velocity
    }
}

impl Default for JumpPad {
    fn default() -> Self {
        Self {
            velocity: Default::default(),
            collider: Default::default(),
        }
    }
}

impl Visit for JumpPad {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.velocity.visit("From", visitor)?;
        self.collider.visit("Shape", visitor)?;

        visitor.leave_region()
    }
}

pub struct JumpPadContainer {
    pool: Pool<JumpPad>,
}

impl Default for JumpPadContainer {
    fn default() -> Self {
        Self::new()
    }
}

impl JumpPadContainer {
    pub fn new() -> Self {
        Self { pool: Pool::new() }
    }

    pub fn add(&mut self, jump_pad: JumpPad) -> Handle<JumpPad> {
        self.pool.spawn(jump_pad)
    }

    pub fn iter(&self) -> impl Iterator<Item = &JumpPad> {
        self.pool.iter()
    }
}

impl Visit for JumpPadContainer {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.pool.visit("Pool", visitor)?;

        visitor.leave_region()
    }
}
