use rg3d_physics::static_geometry::StaticGeometry;
use rg3d_core::{
    pool::{
        Pool,
        PoolIterator,
        Handle
    },
    math::vec3::Vec3,
    visitor::{
        Visit,
        Visitor,
        VisitResult
    }
};
use rg3d_physics::Physics;

pub struct JumpPad {
    force: Vec3,
    shape: Handle<StaticGeometry>
}

impl JumpPad {
    pub fn new(shape: Handle<StaticGeometry>, force: Vec3) -> JumpPad {
        Self {
            force,
            shape
        }
    }

    pub fn get_shape(&self) -> Handle<StaticGeometry> {
        self.shape
    }

    pub fn get_force(&self) -> Vec3 {
        self.force
    }
}

impl Default for JumpPad {
    fn default() -> Self {
        Self {
            force: Default::default(),
            shape: Default::default()
        }
    }
}

impl Visit for JumpPad {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.force.visit("From", visitor)?;
        self.shape.visit("Shape", visitor)?;

        visitor.leave_region()
    }
}

pub struct JumpPadContainer {
    pool: Pool<JumpPad>
}

impl Default for JumpPadContainer {
    fn default() -> Self {
        Self::new()
    }
}

impl JumpPadContainer {
    pub fn new() -> Self {
        Self {
            pool: Pool::new()
        }
    }

    pub fn add(&mut self, jump_pad: JumpPad) -> Handle<JumpPad> {
        self.pool.spawn(jump_pad)
    }

    pub fn iter(&self) -> PoolIterator<JumpPad> {
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