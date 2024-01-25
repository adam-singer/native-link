use std::borrow::BorrowMut;
use std::marker::Send;
use std::pin::Pin;
use std::sync::Arc;

use async_trait::async_trait;
use bytes::Bytes;
use futures::{join, try_join};
use nativelink_error::{Error, ResultExt};
use futures::stream::FuturesUnordered;
use futures::Future;
use futures::FutureExt;
use async_recursion::async_recursion;

use std::fmt::Debug;


#[derive(Debug, Clone, PartialEq, Copy)]
pub enum HealthStatus {
    Ok,
    Initializing,
    Warning,
    Failed,
}

#[async_trait]
pub trait HealthStatusIndicator<'a>: Sync + Send + Unpin {
    async fn check_health(self: Arc<Self>) -> Result<HealthStatus, Error> {
        Ok(HealthStatus::Ok)
    }
}

type HealthComponent = String;
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

    pub fn add_dependency(&mut self, component: HealthComponent) -> &mut Self {
        let dependency = HealthRegistry::new(component);

        self.registries.push(dependency);
        self.registries
            .last_mut()
            .expect("dependencies should not to be empty.")
    }

    #[async_recursion]
    async fn flatten(&mut self,
        //futures: &mut FuturesUnordered<Result<HealthStatus, Error>>,
        results: &mut Vec<HealthStatus>,
        indicators: &Vec<Arc<dyn HealthStatusIndicator<'a>>>, registries: &Vec<HealthRegistry<'a>>) -> Result<(), Error> {
        for indicator in indicators {
            let result = indicator.clone().check_health().await;

            let health_status = match result {
                Ok(health_status) => health_status,
                Err(_) => HealthStatus::Failed,
            };

            results.push(health_status);
        }

        for registry in registries {
            let _ = self.clone().flatten(results, &registry.indicators, &registry.registries).await;
        }

        Ok(())
    }

    pub async fn flatten_indicators(&mut self) -> Vec<HealthStatus> {
        // let mut futures:FuturesUnordered<_>  = FuturesUnordered::new();
        let mut health_status_results = Vec::new();
        let indicators = &self.indicators;
        let registries = &self.registries;
        let _ = self.clone().flatten(&mut health_status_results, indicators, registries).await;
        health_status_results
    }
}
