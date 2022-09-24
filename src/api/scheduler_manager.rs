use std::sync::Arc;

use async_trait::async_trait;
use tokio_cron_scheduler::JobSchedulerError;
use uuid::Uuid;

use crate::behaviour::entity::scheduled_job::ScheduledJob;
use crate::behaviour::entity::timer::Timer;
use crate::plugins::PluginContext;

#[async_trait]
pub trait SchedulerManager: Send + Sync {
    fn init(&self);

    fn shutdown(&self);

    fn set_context(&self, context: Arc<dyn PluginContext>);

    async fn create_scheduler(&self);

    fn start_loop(&self);

    fn remove_job(&self, entity_type: String, id: Uuid, job_id: Uuid) -> Result<(), JobSchedulerError>;

    fn add_scheduled_job_job(&self, id: Uuid);

    fn remove_scheduled_job_job(&self, id: Uuid);

    fn register_scheduled_job(&self, scheduled_job: Arc<ScheduledJob>);

    fn unregister_scheduled_job(&self, scheduled_job: Arc<ScheduledJob>);

    fn add_timer_job(&self, id: Uuid);

    fn remove_timer_job(&self, id: Uuid);

    fn register_timer(&self, timer: Arc<Timer>);

    fn unregister_timer(&self, timer: Arc<Timer>);
}
