use std::sync::{mpsc, Arc, Mutex};

use crate::error::Result;

pub use self::types::{Direction, Request, Response};

mod builder;
mod types;

pub type HookFn = Box<dyn Fn(Request) -> Result<Option<Response>> + Sync + Send>;

pub struct Hook {
    pub direction: Option<Direction>,
    pub target_name: Option<String>,
    pub trigger_fn: HookFn,
}

impl Hook {
    pub fn builder(trigger_fn: HookFn) -> builder::HookBuilder {
        builder::HookBuilder::new(trigger_fn)
    }

    pub fn matches(&self, request: &Request) -> bool {
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
    hooks: Arc<Mutex<Vec<Hook>>>,
    request_receiver: mpsc::Receiver<Request>,
    response_sender: mpsc::Sender<Result<Response>>,
) {
    std::thread::spawn(move || {
        for request in request_receiver {
            let mut data = request.data.clone();
            let hooks = hooks.lock().unwrap();
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

            let response = Response::new(data);
            response_sender
                .send(Ok(response))
                .expect("response_sender is active");
        }
    });
}
