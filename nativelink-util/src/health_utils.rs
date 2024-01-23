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

pub trait HealthStatusIndicator: std::fmt::Debug + Send + Sync + 'static  {
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

    // TODO(adams): do we really need this a a "collect"? Since iterator is implemented here it
    //  could be anything..
    fn collect<'a>(
        &'a self,
    ) -> Box<dyn Iterator<Item = Cow<'a, HealthStatus>> + 'a> {
        let a: HealthStatus = self.internal_check_health();
        let b = Cow::Owned(a);
        let c = vec![b];
        let d = c.into_iter();
        let e = Box::new(d);
        e
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

// pub trait HealthCollectorTrait: std::fmt::Debug + Send + Sync + 'static {
//     fn collect<'a>(
//         &'a self,
//     ) -> Box<dyn Iterator<Item = Cow<'a, HealthStatus>> + 'a>;
// }


impl<T: Sync + Send + 'static> Debug for HealthCollector<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Collector").finish()
    }
}

#[derive(Debug, Default)]
pub struct HealthRegistry {
    health_component: HealthComponent,
    collectors: Vec<Box<dyn HealthStatusIndicator>>,
    dependencies: Vec<HealthRegistry>,
}

impl HealthRegistry {
    pub fn new(health_component: impl Into<String>) -> Self {
        Self {
            health_component: HealthComponent(health_component.into()),
            ..Default::default()
        }
    }

    pub fn register_collector(&mut self, collector: Box<dyn HealthStatusIndicator>) {
        self.collectors.push(collector);
    }

    pub fn add_dependency(&mut self, health_component: impl Into<String>) -> &mut Self {
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
            health_component: Cow::Owned(HealthComponent(self.health_component.as_str().to_string())),
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
    health_component: Cow<'a, HealthComponent>,

    collector: Option<Box<dyn Iterator<Item = Cow<'a, HealthStatusDescription<'a>>> >>,
    collectors: std::slice::Iter<'a, Box<dyn HealthStatusIndicator>>,

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

#[derive(Clone)]
pub struct HealthStatusDescription<'a> {
    health_component: Cow<'a, HealthComponent>,
    health_status: Cow<'a, HealthStatus>,
}

impl<'a> HealthStatusDescription<'a> {
    pub fn new(health_component: Cow<'a, HealthComponent>,
    health_status: Cow<'a, HealthStatus>) -> Self {
        HealthStatusDescription {
            health_component: health_component.clone(),
            health_status: health_status.clone(),
        }
    }
}

impl <'a> Iterator for HealthCollectorIterator<'a> {
    type Item = Cow<'a, HealthStatusDescription<'a>>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {

            if let Some(m) = self
                .collector
                .as_mut()
                .and_then(|c| c.next())
                .or_else(|| self.dependency_collector_iter
                                .as_mut()
                                .and_then(|i| i.next()))
                .map(|health_status_description| {
                    // TODO(adams): addtional struct wrapping
                    // self.health_component
                    // Some(health_status)
                    // Some(HealthStatusDescription {
                    //     health_component: self.health_component,
                    //     health_status: health_status,
                    // })
                    Some(health_status_description)
                })
                {
                    return m;
                }

            if let Some(collector) = self.collectors.next() {
                // let c: dyn Iterator<Item = Cow<'_, HealthStatusDescription<'_>>> = collector

                // .collect()

                // .map(|f| {
                //     // Cow::Owned::<'a>(HealthStatusDescription {
                //     //     health_component: self.health_component.clone(),
                //     //     health_status: f.clone(),
                //     // })
                //     let ss = self.health_component.clone();
                //     let ff = f.clone();
                //    Cow::Owned(HealthStatusDescription::new(ss, ff))
                // }).into();
                // self.collector = Some(Box::new(c));

                let mut v = Vec::new();

                let c = collector.collect();

                for i in c {
                    let ss = self.health_component.clone();
                    let ff = i.clone();
                    let cow: Cow<'_, HealthStatusDescription> = Cow::Owned(HealthStatusDescription::new(ss, ff));
                    v.push(cow);
                }

                self.collector = Some(Box::new(v.into_iter()));
                // self.collector = Some(collector.collect());
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
