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
        // // Initial duration
        // let duration = e.get(TimerProperties::DURATION.as_ref()).and_then(|v| to_duration(&v));
        // // let duration = e.properties.get(TimerProperties::DURATION.as_ref()).and_then(to_duration);
        // if duration.is_none() {
        //     return Err(BehaviourCreationError);
        // }
        // let duration = duration.unwrap();
        //
        // // Construct timer thread
        // let (stopper_tx, stopper_rx) = crossbeam::channel::bounded(1);
        // let (duration_tx, duration_rx) = crossbeam::channel::unbounded();
        // let entity = e.clone();
        // let thread_name = format!("{}-{}", e.type_name.clone(), e.id.to_string());
        // let _handler = task::Builder::new().name(thread_name).spawn(async move {
        //     let mut duration = duration;
        //     loop {
        //         std::thread::sleep(duration);
        //         entity.set(TimerProperties::TRIGGER.as_ref(), json!(true));
        //         // Receive duration change
        //         match duration_rx.try_recv() {
        //             Ok(d) => {
        //                 duration = d;
        //             }
        //             Err(_) => {}
        //         }
        //         // Receive stop event loop
        //         if stopper_rx.try_recv().is_ok() {
        //             break;
        //         }
        //     }
        //     debug!("Timer destroyed");
        // });
        //
        // let handle_id = e.properties.get(TimerProperties::DURATION.as_ref()).unwrap().id.as_u128();
        // e.properties
        //     .get(TimerProperties::DURATION.as_ref())
        //     .unwrap()
        //     .stream
        //     .read()
        //     .unwrap()
        //     .observe_with_handle(
        //         move |duration| {
        //             if let Some(duration) = to_duration(duration) {
        //                 duration_tx.send(duration);
        //             }
        //             // if let Some(duration) = duration.as_u64() {
        //             //     let _ = duration_tx.send(duration);
        //             // }
        //             // if let Some(duration) = duration.as_str() {
        //             //     let _ = duration_tx.send(duration);
        //             // }
        //         },
        //         handle_id,
        //     );
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
    fn disconnect(&self) {
        // // Stop event loop thread
        // let _ = self.stopper_tx.send(());
        // if let Some(property) = self.entity.properties.get(TimerProperties::DURATION.as_ref()) {
        //     trace!("Disconnecting {} with id {}", TIMER, self.entity.id);
        //     property.stream.read().unwrap().remove(self.handle_id);
        // }
    }
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
