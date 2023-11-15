use super::{types::Direction, Hook, HookFn};

pub struct HookBuilder {
    pub direction: Option<Direction>,
    pub target_name: Option<String>,
    pub trigger_fn: HookFn,
}

impl HookBuilder {
    pub fn new(trigger_fn: HookFn) -> Self {
        Self {
            direction: None,
            target_name: None,
            trigger_fn,
        }
    }

    pub fn direction(mut self, direction: Direction) -> Self {
        self.direction = Some(direction);
        self
    }

    pub fn target_name(mut self, target_name: String) -> Self {
        self.target_name = Some(target_name);
        self
    }

    pub fn build(self) -> Hook {
        Hook {
            direction: self.direction,
            target_name: self.target_name,
            trigger_fn: self.trigger_fn,
        }
    }
}
