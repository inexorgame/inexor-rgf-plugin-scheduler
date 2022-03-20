use std::collections::HashMap;
use std::sync::Arc;
use std::sync::RwLock;

use crate::model::PropertyInstanceSetter;
use async_trait::async_trait;
use log::debug;
use log::error;
use log::info;
use serde_json::json;
use tokio::runtime::Runtime;
use tokio::task::JoinHandle;
use tokio_cron_scheduler::JobScheduler;
use tokio_cron_scheduler::{Job, JobSchedulerError};
use uuid::Uuid;

use crate::api::JobWrapper;
use crate::api::SchedulerManager;
use crate::behaviour::entity::scheduled_job::ScheduledJob;
use crate::behaviour::entity::scheduled_job::SCHEDULED_JOB;
use crate::behaviour::entity::timer::Timer;
use crate::behaviour::entity::timer::TIMER;
use crate::behaviour::entity::ScheduledJobProperties;
use crate::behaviour::entity::TimerProperties;
use crate::di::*;
use crate::plugins::PluginContext;

#[wrapper]
pub struct JobSchedulerContainer(JobScheduler);

#[wrapper]
pub struct ScheduledJobStorage(RwLock<HashMap<Uuid, Arc<ScheduledJob>>>);

#[wrapper]
pub struct TimerStorage(RwLock<HashMap<Uuid, Arc<Timer>>>);

#[wrapper]
pub struct PluginContextContainer(RwLock<Option<Arc<dyn PluginContext>>>);

#[wrapper]
pub struct RuntimeContainer(Runtime);

#[provides]
fn create_job_scheduler_container() -> JobSchedulerContainer {
    JobSchedulerContainer(JobScheduler::new())
}

#[provides]
fn create_scheduled_job_storage() -> ScheduledJobStorage {
    ScheduledJobStorage(RwLock::new(HashMap::new()))
}

#[provides]
fn create_timer_storage() -> TimerStorage {
    TimerStorage(RwLock::new(HashMap::new()))
}

#[provides]
fn create_empty_plugin_context_container() -> PluginContextContainer {
    PluginContextContainer(RwLock::new(None))
}

#[provides]
fn create_runtime() -> RuntimeContainer {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_time()
        .thread_name("inexor-scheduler")
        .build()
        .unwrap();
    RuntimeContainer(runtime)
}

#[component]
pub struct SchedulerManagerImpl {
    job_scheduler: JobSchedulerContainer,
    scheduled_jobs: ScheduledJobStorage,
    timers: TimerStorage,
    runtime: RuntimeContainer,
    context: PluginContextContainer,
}

impl SchedulerManagerImpl {}

#[async_trait]
#[provides]
impl SchedulerManager for SchedulerManagerImpl {
    fn init(&self) {
        info!("Starting job scheduler");
        let job_scheduler = self.job_scheduler.0.clone();
        self.runtime.0.spawn(async move {
            // let jl: JobsSchedulerLocked = self.clone();
            let jh: JoinHandle<()> = tokio::spawn(async move {
                loop {
                    tokio::time::sleep(core::time::Duration::from_millis(10)).await;
                    let mut jsl = job_scheduler.clone();
                    let tick = jsl.tick();
                    if let Err(e) = tick {
                        eprintln!("Error on job scheduler tick {:?}", e);
                        break;
                    }
                }
            });
            match jh.await {
                Ok(_) => {
                    info!("Successfully started scheduler");
                }
                Err(err) => {
                    error!("Failed to started scheduler: {:?}", err);
                }
            }
        });
    }

    fn shutdown(&self) {
        debug!("Shutting down job scheduler");
        let mut job_scheduler = self.job_scheduler.0.clone();
        match job_scheduler.shutdown() {
            Ok(_) => {
                info!("Successfully shut down scheduler");
            }
            Err(err) => {
                error!("Failed to shut down scheduler: {:?}", err);
            }
        }
    }

    fn set_context(&self, context: Arc<dyn PluginContext>) {
        self.context.0.write().unwrap().replace(context.clone());
    }

    fn remove_job(&self, entity_type: String, id: Uuid, job_id: Uuid) -> Result<(), JobSchedulerError> {
        let mut job_scheduler = self.job_scheduler.0.clone();
        match job_scheduler.remove(&job_id) {
            Ok(_) => {
                debug!("Removed {} {} from scheduler (job_id: {})", entity_type, id, job_id);
                Ok(())
            }
            Err(err) => {
                error!("Failed to remove {} {} from scheduler (job_id: {}): {:?}", entity_type, id, job_id, err);
                Err(err)
            }
        }
    }

    fn add_scheduled_job_job(&self, id: Uuid) {
        self.remove_scheduled_job_job(id);
        let reader = self.scheduled_jobs.0.read().unwrap();
        if let Some(scheduled_job) = reader.get(&id).cloned() {
            match scheduled_job.get_schedule() {
                Some(schedule) => {
                    let entity = scheduled_job.entity.clone();
                    debug!("Creating job for {} {} with schedule {}", SCHEDULED_JOB, entity.id, schedule.as_str());
                    let job = Job::new(schedule.as_str(), move |job_id, _locked| {
                        debug!("Triggered {} (job_id: {})", SCHEDULED_JOB, job_id);
                        entity.set(ScheduledJobProperties::TRIGGER.as_ref(), json!(true));
                    });
                    match job {
                        Ok(job) => {
                            let job_id = job.guid();
                            let mut job_scheduler = self.job_scheduler.0.clone();
                            match job_scheduler.add(job) {
                                Ok(_) => {
                                    scheduled_job.set_job_id(job_id);
                                    debug!("Successfully scheduled {} {} (job_id: {})", SCHEDULED_JOB, id, job_id);
                                }
                                Err(err) => {
                                    error!("Failed to schedule {} {} (job_id: {}): {:?}", SCHEDULED_JOB, id, job_id, err);
                                }
                            }
                        }
                        Err(err) => {
                            error!("Failed to create job for {} {}: {}", SCHEDULED_JOB, id, err);
                        }
                    }
                }
                None => {
                    error!("Failed to create job for {} {}: Invalid schedule format or missing schedule", SCHEDULED_JOB, id);
                }
            }
        }
    }

    fn remove_scheduled_job_job(&self, id: Uuid) {
        let reader = self.scheduled_jobs.0.read().unwrap();
        if let Some(scheduled_job) = reader.get(&id).cloned() {
            if let Some(job_id) = scheduled_job.get_job_id() {
                if let Ok(()) = self.remove_job(String::from(SCHEDULED_JOB), id, job_id) {
                    scheduled_job.remove_job_id();
                }
            }
        }
    }

    fn register_scheduled_job(&self, scheduled_job: Arc<ScheduledJob>) {
        let id = scheduled_job.entity.id;
        self.scheduled_jobs.0.write().unwrap().insert(id, scheduled_job.clone());
        debug!("Registered {}: {}", SCHEDULED_JOB, id);
        self.add_scheduled_job_job(id);
    }

    fn unregister_scheduled_job(&self, scheduled_job: Arc<ScheduledJob>) {
        let id = scheduled_job.entity.id;
        self.remove_scheduled_job_job(id);
        self.scheduled_jobs.0.write().unwrap().remove(&id);
        debug!("Unregistered {}: {}", SCHEDULED_JOB, id);
    }

    fn add_timer_job(&self, id: Uuid) {
        self.remove_timer_job(id);
        let reader = self.timers.0.read().unwrap();
        if let Some(timer) = reader.get(&id).cloned() {
            match timer.get_duration() {
                Some(duration) => {
                    let mut job_scheduler = self.job_scheduler.0.clone();
                    self.runtime.0.spawn(async move {
                        let entity = timer.entity.clone();
                        debug!("Creating job for {} {} with duration {:?}", TIMER, entity.id, duration);
                        let job = Job::new_repeated(duration, move |job_id, _locked| {
                            debug!("Triggered {} (job_id: {})", TIMER, job_id);
                            entity.set(TimerProperties::TRIGGER.as_ref(), json!(true));
                        });
                        match job {
                            Ok(job) => {
                                let job_id = job.guid();
                                // let mut job_scheduler = self.job_scheduler.0.clone();
                                match job_scheduler.add(job) {
                                    Ok(_) => {
                                        timer.set_job_id(job_id);
                                        debug!("Successfully scheduled {} {} (job_id: {})", TIMER, id, job_id);
                                    }
                                    Err(err) => {
                                        error!("Failed to schedule {} {} (job_id: {}): {:?}", TIMER, id, job_id, err);
                                    }
                                }
                            }
                            Err(err) => {
                                error!("Failed to create job for {} {}: {}", TIMER, id, err);
                            }
                        }
                    });
                }
                None => {
                    error!("Failed to create job for {} {}: Invalid duration format or missing duration", TIMER, id);
                }
            }
        }
    }

    fn remove_timer_job(&self, id: Uuid) {
        let reader = self.timers.0.read().unwrap();
        if let Some(timer) = reader.get(&id).cloned() {
            if let Some(job_id) = timer.get_job_id() {
                if let Ok(()) = self.remove_job(String::from(TIMER), id, job_id) {
                    timer.remove_job_id();
                }
            }
        }
    }

    fn register_timer(&self, timer: Arc<Timer>) {
        let id = timer.entity.id;
        self.timers.0.write().unwrap().insert(id, timer.clone());
        debug!("Registered {}: {}", TIMER, id);
        self.add_timer_job(id);
    }

    fn unregister_timer(&self, timer: Arc<Timer>) {
        let id = timer.entity.id;
        self.remove_timer_job(id);
        self.timers.0.write().unwrap().remove(&id);
        debug!("Unregistered {}: {}", TIMER, id);
    }
}
