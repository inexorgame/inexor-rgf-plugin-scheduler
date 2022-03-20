use std::convert::AsRef;
use std::sync::{Arc, RwLock};

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
        // let entity = e.clone();
        // let handle_id = e.properties.get(ScheduledJobProperties::SCHEDULE.as_ref()).unwrap().id.as_u128();
        // e.properties
        //     .get(ScheduledJobProperties::SCHEDULE.as_ref())
        //     .unwrap()
        //     .stream
        //     .read()
        //     .unwrap()
        //     .observe_with_handle(
        //         move |schedule| {
        //             // if !trigger.is_boolean() || !trigger.as_bool().unwrap() {
        //             //     return;
        //             // }
        //             // entity.set(ScheduledJobProperties::TRIGGER.as_ref(), json!(Uuid::new_v4()));
        //         },
        //         handle_id,
        //     );
        Ok(ScheduledJob {
            entity: e.clone(),
            job_id: RwLock::new(None),
            // handle_id,
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
    fn disconnect(&self) {
        // if let Some(property) = self.entity.properties.get(ScheduledJobProperties::SCHEDULE.as_ref()) {
        //     trace!("Disconnecting {} with id {}", SCHEDULED_JOB, self.entity.id);
        //     property.stream.read().unwrap().remove(self.handle_id);
        // }
    }
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

// impl TryFrom<ScheduledJob> for Job {
//     type Error = ();
//
//     fn try_from(scheduled_job: ScheduledJob) -> Result<Self, Self::Error> {
//         let entity = scheduled_job.entity.clone();
//         match scheduled_job.entity.get(ScheduledJobProperties::SCHEDULE.as_ref()).and_then(Value::as_str) {
//             Some(schedule) => Job::new(schedule, |_uuid| {
//                 entity.set(ScheduledJobProperties::TRIGGER.as_ref(), json!(true));
//             }),
//             None => Self::Error,
//         }
//     }
// }
