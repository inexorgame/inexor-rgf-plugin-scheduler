use std::convert::AsRef;
use std::sync::Arc;
use std::sync::RwLock;

use crate::api::JobWrapper;
use crate::behaviour::entity::ScheduledJobProperties;
use crate::model::ReactiveEntityInstance;
use crate::reactive::entity::Disconnectable;
use crate::reactive::BehaviourCreationError;
use uuid::Uuid;

pub const SCHEDULED_JOB: &str = "scheduled_job";

pub struct ScheduledJob {
    pub entity: Arc<ReactiveEntityInstance>,

    pub job_id: RwLock<Option<Uuid>>,
}

impl ScheduledJob {
    pub fn new<'a>(e: Arc<ReactiveEntityInstance>) -> Result<ScheduledJob, BehaviourCreationError> {
        Ok(ScheduledJob {
            entity: e.clone(),
            job_id: RwLock::new(None),
        })
    }

    pub fn get_schedule(&self) -> Option<String> {
        self.entity
            .properties
            .get(ScheduledJobProperties::SCHEDULE.as_ref())
            .and_then(|p| p.as_string())
    }
}

impl Disconnectable for ScheduledJob {
    fn disconnect(&self) {}
}

impl Drop for ScheduledJob {
    fn drop(&mut self) {
        self.disconnect();
    }
}

impl JobWrapper for ScheduledJob {
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
