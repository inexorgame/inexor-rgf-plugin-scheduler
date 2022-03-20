pub use scheduler_manager::*;
use uuid::Uuid;

pub mod scheduler_manager;

pub trait JobWrapper {
    fn get_job_id(&self) -> Option<Uuid>;

    fn set_job_id(&self, job_id: Uuid);

    fn remove_job_id(&self);
}
