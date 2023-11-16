#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Direction {
    #[default]
    ClientToTarget,
    TargetToClient,
}

impl Direction {
    pub fn from_client(&self) -> bool {
        matches!(self, Self::ClientToTarget)
    }

    pub fn from_target(&self) -> bool {
        matches!(self, Self::TargetToClient)
    }

    pub fn to_client(&self) -> bool {
        matches!(self, Self::TargetToClient)
    }

    pub fn to_target(&self) -> bool {
        matches!(self, Self::ClientToTarget)
    }
}

#[derive(Debug, Clone)]
pub struct HookRequest {
    pub direction: Direction,
    pub target_name: String,
    pub data: Box<[u8]>,
}

impl HookRequest {
    pub fn new(direction: Direction, target_name: impl Into<String>, data: Box<[u8]>) -> Self {
        Self {
            direction,
            target_name: target_name.into(),
            data,
        }
    }
}

#[derive(Debug)]
pub struct HookResponse {
    pub data: Box<[u8]>,
}

impl HookResponse {
    pub fn new(data: Box<[u8]>) -> Self {
        Self { data }
    }
}
