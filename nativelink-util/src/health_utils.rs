use std::marker::PhantomData;
use std::sync::{Arc, Weak};
use std::borrow::{BorrowMut, Cow};
use std::fmt::Debug;

type Name = Cow<'static, str>;
pub type Description = Cow<'static, str>;

#[derive(Debug, Clone)]
pub enum HealthStatus {
    Ok(Name,  Description),
    Initializing(Name,  Description),
    Warning(Name,  Description),
    Failed(Name,  Description),
}

pub trait HealthStatusIndicator {
    fn component_name(&self) -> Name {
        Cow::Borrowed(std::any::type_name::<Self>())
    }

    // https://play.rust-lang.org/?version=nightly&mode=debug&edition=2021&gist=ed4a7ab0f40adbe5166f4fa016b36276
    fn internal_check_health(&self) -> HealthStatus {
        HealthStatus::Ok(self.component_name(), self.check_health())
    }

    // TODO: Most likely this will need to be async
    fn check_health(&self) -> Description {
        "no error".into()
    }
}


pub struct HealthCollector<T>
where T: Sync + Send + 'static,
{
    handle: Weak<T>,
    _marker: PhantomData<T>,
}

impl<T> HealthCollector<T>
where T: Sync + Send + 'static,
{
    pub fn new(handle: &Arc<T>) -> Self {
        Self {
            handle: Arc::downgrade(handle),
            _marker: PhantomData,
        }
    }
}

pub trait HealthCollectorTrait: std::fmt::Debug + Send + Sync + 'static {
    fn collect<'a>(
        &'a self,
    ) -> Box<dyn Iterator<Item = Cow<'a, HealthStatus>> + 'a>;
}


impl<T: Sync + Send + 'static> Debug for HealthCollector<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Collector").finish()
    }
}

#[derive(Debug, Default)]
pub struct HealthRegistry {
    health_component: HealthComponent,
    collectors: Vec<Box<dyn HealthCollectorTrait>>,
    dependencies: Vec<HealthRegistry>,
}

impl HealthRegistry {
    pub fn new(health_component: impl Into<String>) -> Self {
        Self {
            health_component: HealthComponent(health_component.into()),
            ..Default::default()
        }
    }

    pub fn register_collector(&mut self, collector: Box<dyn HealthCollectorTrait>) {
        self.collectors.push(collector);
    }

    pub fn add_dependency<H: AsRef<str>>(&mut self, health_component: impl Into<String>) -> &mut Self {
        let dependency = HealthRegistry {
            health_component: HealthComponent(health_component.into()), // NOTE: we could do some name munging here if we wanted.
            ..Default::default()
        };

        self.dependencies.push(dependency);
        self.dependencies.last_mut().expect("dependencies should not to be empty.")
    }

    pub fn iter_collectors(&self) -> HealthCollectorIterator {
        let collectors = self.collectors.iter();
        let dependency_registries = self.dependencies.iter();
        HealthCollectorIterator {
            // TODO(adams): fix the conversion here.
            health_component: HealthComponent(self.health_component.as_str().to_string()),
            collector: None,
            collectors: collectors,
            dependency_collector_iter: None,
            dependency_registries: dependency_registries
        }
    }
}


#[derive(Debug, Clone, Default)]
pub struct HealthComponent(String);
impl HealthComponent {
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl From<String> for HealthComponent {
    fn from(s: String) -> Self {
        HealthComponent(s)
    }
}


pub struct HealthCollectorIterator<'a> {
    health_component: HealthComponent,

    collector: Option<Box<dyn Iterator<Item = Cow<'a, HealthStatus>> + 'a>>,
    collectors: std::slice::Iter<'a, Box<dyn HealthCollectorTrait>>,

    dependency_collector_iter: Option<Box<HealthCollectorIterator<'a>>>,
    dependency_registries: std::slice::Iter<'a, HealthRegistry>,
}

impl<'a> std::fmt::Debug for HealthCollectorIterator<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HealthCollectorIterator")
            .field("health_component", &self.health_component)
            .finish()
    }
}

impl <'a> Iterator for HealthCollectorIterator<'a> {
    type Item = Cow<'a, HealthStatus>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {

            if let Some(m) = self
                .collector
                .as_mut()
                .and_then(|c| c.next())
                .or_else(|| self.dependency_collector_iter
                                .as_mut()
                                .and_then(|i| i.next()))
                .map(|health_status| {
                    // TODO(adams): addtional struct wrapping
                    Some(health_status)
                })
                {
                    return m;
                }

            if let Some(collector) = self.collectors.next() {
                self.collector = Some(collector.collect());
                continue;
            }

            if let Some(collector_iter) = self.dependency_registries
            .next()
            .map(|r| Box::new(r.iter_collectors()))
            {
                self.dependency_collector_iter = Some(collector_iter);
                continue;
            }

            return None;
        }
    }
}
