use std::sync::{mpsc, Arc};

use crate::error::Result;

pub use self::types::{Direction, HookRequest, HookResponse};

mod builder;
mod types;

pub type HookFn = Box<dyn Fn(HookRequest) -> Result<Option<HookResponse>> + Sync + Send>;

pub struct Hook {
    pub direction: Option<Direction>,
    pub target_name: Option<String>,
    pub trigger_fn: HookFn,
}

impl Hook {
    pub fn builder(trigger_fn: HookFn) -> builder::HookBuilder {
        builder::HookBuilder::new(trigger_fn)
    }

    pub fn matches(&self, request: &HookRequest) -> bool {
        if let Some(direction) = self.direction {
            if direction != request.direction {
                return false;
            }
        }

        if let Some(target_name) = &self.target_name {
            if target_name != &request.target_name {
                return false;
            }
        }

        true
    }
}

pub struct Header {
    pub direction: Direction,
}

pub fn hook_executor(
    hooks: Arc<Vec<Hook>>,
    request_receiver: mpsc::Receiver<HookRequest>,
    response_sender: mpsc::Sender<Result<HookResponse>>,
) {
    std::thread::spawn(move || {
        for request in request_receiver {
            let mut data = request.data.clone();
            for hook in hooks.iter().filter(|h| h.matches(&request)) {
                data = match (hook.trigger_fn)(request.clone()) {
                    Ok(Some(response)) => response.data.clone(),
                    Ok(None) => data,
                    Err(err) => {
                        eprintln!("Error running hook: {:?}", err);
                        response_sender
                            .send(Err(err))
                            .expect("response_sender is active");
                        return;
                    }
                }
            }

            let response = HookResponse::new(data);
            response_sender
                .send(Ok(response))
                .expect("response_sender is active");
        }
    });
}
