use rg3d::{
    core::{
        algebra::Vector3,
        pool::{Handle, Pool, PoolIterator},
        visitor::{Visit, VisitResult, Visitor},
    },
    scene::RigidBodyHandle,
};

pub struct JumpPad {
    force: Vector3<f32>,
    body: RigidBodyHandle,
}

impl JumpPad {
    pub fn new(shape: RigidBodyHandle, force: Vector3<f32>) -> JumpPad {
        Self { force, body: shape }
    }

    pub fn rigid_body(&self) -> RigidBodyHandle {
        self.body
    }

    pub fn get_force(&self) -> Vector3<f32> {
        self.force
    }
}

impl Default for JumpPad {
    fn default() -> Self {
        Self {
            force: Default::default(),
            body: Default::default(),
        }
    }
}

impl Visit for JumpPad {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.force.visit("From", visitor)?;
        self.body.visit("Shape", visitor)?;

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
