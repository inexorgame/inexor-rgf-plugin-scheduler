use std::collections::HashMap;
use std::sync::Arc;
use std::sync::RwLock;

use crate::api::SchedulerManager;
use crate::di::*;
use crate::model::ReactiveBehaviourContainer;
use async_trait::async_trait;
use log::debug;
use uuid::Uuid;

use crate::behaviour::entity::scheduled_job::ScheduledJob;
use crate::behaviour::entity::scheduled_job::SCHEDULED_JOB;
use crate::behaviour::entity::timer::Timer;
use crate::behaviour::entity::timer::TIMER;
use crate::behaviour::entity::{ScheduledJobProperties, TimerProperties};
use crate::model::ReactiveEntityInstance;
use crate::plugins::EntityBehaviourProvider;

#[wrapper]
pub struct ScheduledJobStorage(RwLock<HashMap<Uuid, Arc<ScheduledJob>>>);

#[wrapper]
pub struct TimerStorage(RwLock<HashMap<Uuid, Arc<Timer>>>);

#[provides]
fn create_scheduled_job_storage() -> ScheduledJobStorage {
    ScheduledJobStorage(RwLock::new(HashMap::new()))
}

#[provides]
fn create_timer_storage() -> TimerStorage {
    TimerStorage(RwLock::new(HashMap::new()))
}

#[async_trait]
pub trait SchedulerEntityBehaviourProvider: EntityBehaviourProvider + Send + Sync {
    fn create_scheduled_job(&self, entity_instance: Arc<ReactiveEntityInstance>);

    fn create_timer(&self, entity_instance: Arc<ReactiveEntityInstance>);

    fn remove_scheduled_job(&self, entity_instance: Arc<ReactiveEntityInstance>);

    fn remove_timer(&self, entity_instance: Arc<ReactiveEntityInstance>);

    fn remove_by_id(&self, id: Uuid);
}

#[module]
pub struct SchedulerEntityBehaviourProviderImpl {
    scheduler_manager: Wrc<dyn SchedulerManager>,

    scheduled_jobs: ScheduledJobStorage,
    timers: TimerStorage,
}

interfaces!(SchedulerEntityBehaviourProviderImpl: dyn EntityBehaviourProvider);

impl SchedulerEntityBehaviourProviderImpl {}

#[async_trait]
#[provides]
impl SchedulerEntityBehaviourProvider for SchedulerEntityBehaviourProviderImpl {
    fn create_scheduled_job(&self, entity_instance: Arc<ReactiveEntityInstance>) {
        let id = entity_instance.id;
        match ScheduledJob::new(entity_instance.clone()) {
            Ok(scheduled_job) => {
                let scheduled_job = Arc::new(scheduled_job);
                self.scheduled_jobs.0.write().unwrap().insert(id, scheduled_job.clone());
                entity_instance.add_behaviour(SCHEDULED_JOB);
                debug!("Added behaviour {} to entity instance {}", SCHEDULED_JOB, id);
                self.scheduler_manager.register_scheduled_job(scheduled_job.clone());
                // Watch cron expression
                let scheduler_manager = self.scheduler_manager.clone();
                scheduled_job
                    .entity
                    .properties
                    .get(ScheduledJobProperties::SCHEDULE.as_ref())
                    .unwrap()
                    .stream
                    .read()
                    .unwrap()
                    .observe_with_handle(
                        move |_| {
                            scheduler_manager.add_scheduled_job_job(id);
                        },
                        id.as_u128(),
                    );
            }
            _ => {}
        }
    }

    fn create_timer(&self, entity_instance: Arc<ReactiveEntityInstance>) {
        let id = entity_instance.id;
        match Timer::new(entity_instance.clone()) {
            Ok(timer) => {
                let timer = Arc::new(timer);
                self.timers.0.write().unwrap().insert(id, timer.clone());
                entity_instance.add_behaviour(TIMER);
                debug!("Added behaviour {} to entity instance {}", TIMER, id);
                self.scheduler_manager.register_timer(timer.clone());
                // Watch duration
                let scheduler_manager = self.scheduler_manager.clone();
                timer
                    .entity
                    .properties
                    .get(TimerProperties::DURATION.as_ref())
                    .unwrap()
                    .stream
                    .read()
                    .unwrap()
                    .observe_with_handle(
                        move |_| {
                            scheduler_manager.add_timer_job(id);
                        },
                        id.as_u128(),
                    );
            }
            _ => {}
        }
    }

    fn remove_scheduled_job(&self, entity_instance: Arc<ReactiveEntityInstance>) {
        let id = entity_instance.id;
        if let Some(scheduled_job) = self.scheduled_jobs.0.write().unwrap().remove(&entity_instance.id) {
            // Unregister scheduled job
            self.scheduler_manager.unregister_scheduled_job(scheduled_job.clone());
            // Remove watcher
            scheduled_job
                .entity
                .properties
                .get(ScheduledJobProperties::SCHEDULE.as_ref())
                .unwrap()
                .stream
                .read()
                .unwrap()
                .remove(id.as_u128());
        }
        entity_instance.remove_behaviour(SCHEDULED_JOB);
        debug!("Removed behaviour {} from entity instance {}", SCHEDULED_JOB, entity_instance.id);
    }

    fn remove_timer(&self, entity_instance: Arc<ReactiveEntityInstance>) {
        let id = entity_instance.id;
        if let Some(timer) = self.timers.0.write().unwrap().remove(&id) {
            // Unregister timer
            self.scheduler_manager.unregister_timer(timer.clone());
            // Remove watcher
            timer
                .entity
                .properties
                .get(TimerProperties::DURATION.as_ref())
                .unwrap()
                .stream
                .read()
                .unwrap()
                .remove(id.as_u128());
        }
        entity_instance.remove_behaviour(TIMER);
        debug!("Removed behaviour {} from entity instance {}", TIMER, entity_instance.id);
    }

    fn remove_by_id(&self, id: Uuid) {
        if self.scheduled_jobs.0.write().unwrap().contains_key(&id) {
            self.scheduled_jobs.0.write().unwrap().remove(&id);
            debug!("Removed behaviour {} from entity instance {}", SCHEDULED_JOB, id);
        }
        if self.timers.0.write().unwrap().contains_key(&id) {
            self.timers.0.write().unwrap().remove(&id);
            debug!("Removed behaviour {} from entity instance {}", TIMER, id);
        }
    }
}

impl EntityBehaviourProvider for SchedulerEntityBehaviourProviderImpl {
    fn add_behaviours(&self, entity_instance: Arc<ReactiveEntityInstance>) {
        match entity_instance.type_name.as_str() {
            SCHEDULED_JOB => self.create_scheduled_job(entity_instance),
            TIMER => self.create_timer(entity_instance),
            _ => {}
        }
    }

    fn remove_behaviours(&self, entity_instance: Arc<ReactiveEntityInstance>) {
        match entity_instance.clone().type_name.as_str() {
            SCHEDULED_JOB => self.remove_scheduled_job(entity_instance),
            TIMER => self.remove_timer(entity_instance),
            _ => {}
        }
    }

    fn remove_behaviours_by_id(&self, id: Uuid) {
        self.remove_by_id(id);
    }
}
