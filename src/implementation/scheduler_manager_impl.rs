use std::collections::HashMap;
use std::sync::Arc;
use std::sync::RwLock;

use async_trait::async_trait;
use log::debug;
use log::error;
use log::info;
use serde_json::json;
use tokio::runtime::Runtime;
use tokio_cron_scheduler::Job;
use tokio_cron_scheduler::JobScheduler;
use tokio_cron_scheduler::JobSchedulerError;
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
use crate::model::PropertyInstanceSetter;
use crate::plugins::PluginContext;

#[wrapper]
pub struct JobSchedulerContainer(RwLock<Option<JobScheduler>>);

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
    JobSchedulerContainer(RwLock::new(None))
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
        .enable_all()
        // .enable_time()
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
        self.runtime.0.block_on(async {
            self.create_scheduler().await;
        });
        self.start_loop();
    }

    fn shutdown(&self) {
        debug!("Shutting down job scheduler");
        let lock_gard = self.job_scheduler.0.read().unwrap();
        if let Some(job_scheduler) = lock_gard.clone() {
            let mut job_scheduler = job_scheduler.clone();
            self.runtime.0.spawn(async move {
                match job_scheduler.shutdown().await {
                    Ok(_) => {
                        info!("Successfully shut down job scheduler");
                    }
                    Err(err) => {
                        error!("Failed to shut down job scheduler: {:?}", err);
                    }
                }
            });
        }
    }

    fn set_context(&self, context: Arc<dyn PluginContext>) {
        self.context.0.write().unwrap().replace(context.clone());
    }

    async fn create_scheduler(&self) {
        if let Ok(job_scheduler) = JobScheduler::new().await {
            let mut lock_gard = self.job_scheduler.0.write().unwrap();
            lock_gard.replace(job_scheduler);
        }
    }

    fn start_loop(&self) {
        let lock_gard = self.job_scheduler.0.read().unwrap();
        if let Some(mut job_scheduler) = lock_gard.clone() {
            self.runtime.0.spawn(async move {
                match job_scheduler.init().await {
                    Ok(_) => {
                        debug!("Successfully initialized job scheduler");
                    }
                    Err(e) => {
                        error!("Failed to init job scheduler: {}", e);
                    }
                }
                match job_scheduler.start().await {
                    Ok(_) => {
                        debug!("Successfully started job scheduler");
                    }
                    Err(e) => {
                        error!("Failed to start job scheduler: {}", e);
                    }
                }
                // tokio::spawn(async move {
                //     let job_scheduler = job_scheduler.clone();
                //     job_scheduler.start();
                // });
                // tokio::spawn(async move {
                //     let job_scheduler = job_scheduler.clone();
                //     loop {
                //         // 100 fps
                //         // tokio::time::sleep(core::time::Duration::from_millis(10)).await;
                //         thread::sleep(Duration::from_millis(10));
                //         match job_scheduler.tick().await {
                //             Ok(_) => {
                //                 // debug!("Tick");
                //             }
                //             Err(e) => {
                //                 error!("Error on job scheduler tick {:?}", e);
                //                 thread::sleep(Duration::from_secs(1));
                //                 // break;
                //             }
                //         }
                //     }
                //     error!("Exited job scheduler loop");
                // });
            });
        }
    }

    fn remove_job(&self, entity_type: String, id: Uuid, job_id: Uuid) -> Result<(), JobSchedulerError> {
        let lock_gard = self.job_scheduler.0.read().unwrap();
        if let Some(job_scheduler) = lock_gard.clone() {
            let job_scheduler = job_scheduler.clone();
            self.runtime.0.block_on(async move {
                match job_scheduler.remove(&job_id).await {
                    Ok(_) => {
                        debug!("Removed {} {} from scheduler (job_id: {})", entity_type, id, job_id);
                    }
                    Err(err) => {
                        error!("Failed to remove {} {} from scheduler (job_id: {}): {:?}", entity_type, id, job_id, err);
                    }
                }
            });
        }
        Ok(())
    }

    fn add_scheduled_job_job(&self, id: Uuid) {
        let lock_gard = self.job_scheduler.0.read().unwrap();
        if let Some(job_scheduler) = lock_gard.clone() {
            let job_scheduler = job_scheduler.clone();
            self.remove_scheduled_job_job(id);
            let reader = self.scheduled_jobs.0.read().unwrap();
            if let Some(scheduled_job) = reader.get(&id).cloned() {
                match scheduled_job.get_schedule() {
                    Some(schedule) => {
                        // let entity_id = scheduled_job.entity.id;
                        let entity = scheduled_job.entity.clone();
                        debug!("Creating job for {} {} with schedule {}", SCHEDULED_JOB, entity.id, schedule.as_str());
                        let job = Job::new(schedule.as_str(), move |job_id, _locked| {
                            debug!("Triggered {} {} (job_id: {})", SCHEDULED_JOB, id, job_id);
                            entity.set(ScheduledJobProperties::TRIGGER.as_ref(), json!(true));
                        });
                        match job {
                            Ok(job) => {
                                self.runtime.0.block_on(async move {
                                    let job_id = job.guid();
                                    match job_scheduler.add(job).await {
                                        Ok(uuid) => {
                                            scheduled_job.set_job_id(uuid);
                                            debug!("Successfully scheduled {} {} (job_id: {})", SCHEDULED_JOB, id, uuid);
                                        }
                                        Err(err) => {
                                            error!("Failed to schedule {} {} (job_id: {}): {:?}", SCHEDULED_JOB, id, job_id, err);
                                        }
                                    }
                                });
                            }
                            Err(e) => {
                                error!("Failed to create job for {} {}: {}", SCHEDULED_JOB, id, e);
                            }
                        }
                    }
                    None => {
                        error!("Failed to create job for {} {}: Invalid schedule format or missing schedule", SCHEDULED_JOB, id);
                    }
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
        let lock_gard = self.job_scheduler.0.read().unwrap();
        if let Some(job_scheduler) = lock_gard.clone() {
            let job_scheduler = job_scheduler.clone();
            self.remove_timer_job(id);
            let reader = self.timers.0.read().unwrap();
            if let Some(timer) = reader.get(&id).cloned() {
                match timer.get_duration() {
                    Some(duration) => {
                        let entity = timer.entity.clone();
                        let job = Job::new_repeated(duration, move |job_id, _locked| {
                            debug!("Triggered {} {} (job_id: {})", TIMER, id, job_id);
                            entity.set(TimerProperties::TRIGGER.as_ref(), json!(true));
                        });
                        match job {
                            Ok(job) => {
                                self.runtime.0.block_on(async move {
                                    let job_id = job.guid();
                                    match job_scheduler.add(job).await {
                                        Ok(uuid) => {
                                            timer.set_job_id(uuid);
                                            info!("Successfully scheduled {} {} (job_id: {})", TIMER, id, uuid);
                                        }
                                        Err(e) => {
                                            error!("Failed to schedule {} {} (job_id: {}): {:?}", TIMER, id, job_id, e);
                                        }
                                    }
                                });
                            }
                            Err(e) => {
                                error!("Failed to create job for {} {}: {}", TIMER, id, e);
                            }
                        }
                    }
                    None => {
                        error!("Failed to create job for {} {}: Invalid duration format or missing duration", TIMER, id);
                    }
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
