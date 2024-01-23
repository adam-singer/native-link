use std::borrow::{BorrowMut, Cow};
use std::fmt::Debug;
use std::marker::PhantomData;
use std::ops::Deref;
use std::sync::{Arc, Weak};

type Name = Cow<'static, str>;
pub type Description = Cow<'static, str>;

#[derive(Debug, Clone)]
pub enum HealthStatus {
    Ok(Name, Description),
    Initializing(Name, Description),
    Warning(Name, Description),
    Failed(Name, Description),
}

pub trait HealthStatusIndicator: std::fmt::Debug + Send + Sync + 'static {
    fn component_name(&self) -> Name {
        Cow::Borrowed(std::any::type_name::<Self>())
    }

    // TODO(adams): we could have something that checks multiple health points instead of single point.
    fn check_health(&self) -> HealthStatus;

    // TODO(adams): internal helper methods or implicits for health status
}

type HealthComponent = Cow<'static, str>;

#[derive(Debug, Default)]
pub struct HealthRegistry {
    health_component: HealthComponent,
    collectors: Vec<Box<dyn HealthStatusIndicator>>,
    dependencies: Vec<HealthRegistry>,
}

impl HealthRegistry {
    pub fn new(health_component: HealthComponent) -> Self {
        Self {
            health_component: health_component,
            ..Default::default()
        }
    }

    pub fn register_collector(&mut self, collector: Box<dyn HealthStatusIndicator>) {
        self.collectors.push(collector);
    }

    pub fn add_dependency(&mut self, health_component: HealthComponent) -> &mut Self {
        let dependency = HealthRegistry {
            health_component: health_component, // NOTE: we could do some name munging here if we wanted.
            ..Default::default()
        };

        self.dependencies.push(dependency);
        self.dependencies
            .last_mut()
            .expect("dependencies should not to be empty.")
    }

    pub fn iter_collectors(&self) -> HealthCollectorIterator {
        let collectors = self.collectors.iter();
        let dependency_registries = self.dependencies.iter();
        HealthCollectorIterator {
            // TODO(adams): fix the conversion here.
            health_component: self.health_component.clone(),
            collector: None,
            collectors: collectors,
            dependency_collector_iter: None,
            dependency_registries: dependency_registries,
        }
    }
}

pub struct HealthCollectorIterator<'a> {
    health_component: HealthComponent,

    collector: Option<Box<dyn Iterator<Item = HealthStatusDescription>>>,
    collectors: std::slice::Iter<'a, Box<dyn HealthStatusIndicator>>,

    dependency_collector_iter: Option<Box<HealthCollectorIterator<'a>>>,
    dependency_registries: std::slice::Iter<'a, HealthRegistry>,
}

impl<'a> Iterator for HealthCollectorIterator<'a> {
    type Item = HealthStatusDescription;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(m) = self
            .collector
            .as_mut()
            .and_then(|c| c.next())
            .or_else(|| self.dependency_collector_iter
                        .as_mut()
                        .and_then(|i| i.next())
            ).map(|h| {
                Some(h)

            }) {
                return m;
            }

            if let Some(collector) = self.collectors.next() {

                let h = collector.check_health();
                let hc = self.health_component.clone();
                let hsd = HealthStatusDescription {
                    health_component: hc,
                    health_status: h,
                };
                self.collector = Some(Box::new(vec![hsd].into_iter()));
                continue;
            }

            if let Some(collector_iter) = self.dependency_registries.next()
            .map(|r| Box::new(r.iter_collectors())) {
                self.dependency_collector_iter = Some(collector_iter);
                continue;
            }

            return None;
        }
    }
}

#[derive(Clone, Debug)]
pub struct HealthStatusDescription {
    health_component: HealthComponent,
    health_status: HealthStatus,
}
