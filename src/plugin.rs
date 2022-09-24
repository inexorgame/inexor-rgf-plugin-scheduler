use std::sync::{Arc, RwLock};

use crate::di::*;
use async_trait::async_trait;

use crate::api::SchedulerManager;
use crate::behaviour::entity::entity_behaviour_provider::SchedulerEntityBehaviourProviderImpl;
use crate::plugins::plugin::PluginMetadata;
use crate::plugins::plugin_context::PluginContext;
use crate::plugins::{EntityBehaviourProvider, EntityTypeProvider, Plugin};
use crate::provider::SchedulerEntityTypeProviderImpl;
use inexor_rgf_core_plugins::plugin::PluginMetadataError;
use inexor_rgf_core_plugins::{
    entity_behaviour_provider, entity_type_provider, plugin_metadata, EntityBehaviourProviderError, EntityTypeProviderError, PluginContextInitializationError,
    PluginPostInitializationError, PluginPreShutdownError,
};
use std::env;

#[wrapper]
pub struct PluginContextContainer(RwLock<Option<std::sync::Arc<dyn PluginContext>>>);

#[provides]
fn create_empty_plugin_context_container() -> PluginContextContainer {
    return PluginContextContainer(RwLock::new(None));
}

#[async_trait]
pub trait SchedulerPlugin: Plugin + Send + Sync {}

#[module]
pub struct SchedulerPluginImpl {
    entity_type_provider: Wrc<SchedulerEntityTypeProviderImpl>,
    entity_behaviour_provider: Wrc<SchedulerEntityBehaviourProviderImpl>,

    scheduler_manager: Wrc<dyn SchedulerManager>,

    context: PluginContextContainer,
}

impl SchedulerPluginImpl {}

interfaces!(SchedulerPluginImpl: dyn Plugin);

#[async_trait]
#[provides]
impl SchedulerPlugin for SchedulerPluginImpl {}

impl Plugin for SchedulerPluginImpl {
    fn metadata(&self) -> Result<PluginMetadata, PluginMetadataError> {
        plugin_metadata!()
    }

    fn post_init(&self) -> Result<(), PluginPostInitializationError> {
        self.scheduler_manager.init();
        Ok(())
    }

    fn pre_shutdown(&self) -> Result<(), PluginPreShutdownError> {
        self.scheduler_manager.shutdown();
        Ok(())
    }

    fn set_context(&self, context: Arc<dyn PluginContext>) -> Result<(), PluginContextInitializationError> {
        self.context.0.write().unwrap().replace(context.clone());
        Ok(())
    }

    fn get_entity_type_provider(&self) -> Result<Option<Arc<dyn EntityTypeProvider>>, EntityTypeProviderError> {
        entity_type_provider!(self.entity_type_provider)
    }

    fn get_entity_behaviour_provider(&self) -> Result<Option<Arc<dyn EntityBehaviourProvider>>, EntityBehaviourProviderError> {
        entity_behaviour_provider!(self.entity_behaviour_provider)
    }
}
