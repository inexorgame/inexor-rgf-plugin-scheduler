use std::convert::AsRef;
use std::sync::Arc;
use std::sync::RwLock;
use std::time::Duration as StdDuration;

use crate::api::JobWrapper;
use iso8601_duration::Duration;
use log::error;
use serde_json::Value;
use uuid::Uuid;

use crate::behaviour::entity::TimerProperties;
use crate::model::PropertyInstanceGetter;
use crate::model::ReactiveEntityInstance;
use crate::reactive::entity::Disconnectable;
use crate::reactive::BehaviourCreationError;

pub const TIMER: &str = "timer";

pub struct Timer {
    pub entity: Arc<ReactiveEntityInstance>,

    pub job_id: RwLock<Option<Uuid>>,
}

impl Timer {
    pub fn new<'a>(e: Arc<ReactiveEntityInstance>) -> Result<Timer, BehaviourCreationError> {
        Ok(Timer {
            entity: e.clone(),
            job_id: RwLock::new(None),
        })
    }

    pub fn get_duration(&self) -> Option<StdDuration> {
        self.entity.get(TimerProperties::DURATION.as_ref()).and_then(|v| to_duration(&v))
    }
}

impl Disconnectable for Timer {
    fn disconnect(&self) {}
}

impl Drop for Timer {
    fn drop(&mut self) {
        self.disconnect();
    }
}

impl JobWrapper for Timer {
    fn get_job_id(&self) -> Option<Uuid> {
        self.job_id.read().unwrap().clone()
    }

    fn set_job_id(&self, job_id: Uuid) {
        self.job_id.write().unwrap().replace(job_id);
    }

    fn remove_job_id(&self) {
        self.job_id.write().unwrap().take();
    }
}

fn to_duration(duration: &Value) -> Option<StdDuration> {
    if let Some(duration) = duration.as_u64() {
        return Some(StdDuration::from_millis(duration));
    }
    if let Some(duration) = duration.as_str() {
        return match Duration::parse(duration) {
            Ok(duration) => Some(duration.to_std()),
            Err(err) => {
                error!("Invalid format {:?}", err);
                None
            }
        };
    }
    None
}
