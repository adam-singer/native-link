use std::borrow::Cow;
use std::marker::Send;
use std::sync::Arc;

use async_recursion::async_recursion;
use async_trait::async_trait;
use nativelink_error::Error;

use std::fmt::Debug;

type HealthComponent = String;
type TypeName = Cow<'static, str>;

#[derive(Debug, Clone, PartialEq, Copy)]
pub enum HealthStatus {
    Ok,
    Initializing,
    Warning,
    Failed,
}

#[derive(Clone, Debug, PartialEq)]
pub struct HealthStatusDescription {
    pub component: HealthComponent,
    pub status: HealthStatus
}

#[async_trait]
pub trait HealthStatusIndicator<'a>: Sync + Send + Unpin {
    // fn type_name(&self) -> TypeName {
    //     Cow::Borrowed(std::any::type_name::<Self>())
    // }

    async fn check_health(self: Arc<Self>) -> Result<HealthStatus, Error> {
        Ok(HealthStatus::Ok)
    }
}


#[derive(Default, Clone)]
pub struct HealthRegistry<'a> {
    component: HealthComponent,
    indicators: Vec<Arc<dyn HealthStatusIndicator<'a>>>,
    registries: Vec<HealthRegistry<'a>>,
}

impl<'a> HealthRegistry<'a> {
    pub fn new(component: HealthComponent) -> Self {
        Self {
            component,
            ..Default::default()
        }
    }

    pub fn register_indicator(&mut self, indicator: Arc<dyn HealthStatusIndicator<'a>>) {
        self.indicators.push(indicator);
    }

    pub fn add_dependency(&mut self, component: HealthComponent) -> &mut HealthRegistry<'a> {
        let dependency = HealthRegistry::new(component);

        self.registries.push(dependency);
        self.registries
            .last_mut()
            .expect("dependencies should not to be empty.")
    }

    #[async_recursion]
    async fn flatten(
        &mut self,
        results: &mut Vec<HealthStatusDescription>,
        parent_component: &HealthComponent,
        component: &HealthComponent,
        indicators: &Vec<Arc<dyn HealthStatusIndicator<'a>>>,
        registries: &Vec<HealthRegistry<'a>>,
    ) -> Result<(), Error> {
        let component_name = &format!("{parent_component}/{component}");
        for indicator in indicators {
            let result = indicator.clone().check_health().await;

            let health_status = match result {
                Ok(health_status) => HealthStatusDescription {
                    component: component_name.clone(),
                    status: health_status
                },
                Err(_) => HealthStatusDescription {
                    component: component_name.clone(),
                    status: HealthStatus::Failed
                },
            };

            results.push(health_status);
        }

        for registry in registries {
            let _ = self
                .clone()
                .flatten(results, &component_name, &registry.component, &registry.indicators, &registry.registries)
                .await;
        }

        Ok(())
    }

    pub async fn flatten_indicators(&mut self) -> Vec<HealthStatusDescription> {
        let mut health_status_results = Vec::new();
        let parent_component: HealthComponent = "".into();
        let component = &self.component;
        let indicators = &self.indicators;
        let registries = &self.registries;
        let _ = self
            .clone()
            .flatten(&mut health_status_results, &parent_component, &component, indicators, registries)
            .await;
        health_status_results
    }
}
