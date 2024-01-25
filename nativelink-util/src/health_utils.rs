use std::borrow::{Borrow, BorrowMut, Cow};
use std::fmt::Debug;
use std::marker::PhantomData;
use std::ops::Deref;
use std::sync::{Arc, Weak};
use async_trait::async_trait;
use futures::stream::FuturesUnordered;
use nativelink_error::Error;
use std::pin::Pin;
use futures::FutureExt;
use futures::stream::{ StreamExt};
use futures::Future;
use tokio::sync::Mutex;

// https://github.com/rust-lang/rust/issues/108345

// type Name = Cow<'static, str>;
// pub type Description = Cow<'static, str>;

type Name<'a> = Cow<'a, str>;
pub type Description<'a> = Cow<'a, str>;

// type Name = String;
// pub type Description = String;

#[derive(Debug, Clone)]
pub enum HealthStatus2<'a> {
    Ok(Name<'a>, Description<'a>),
    Initializing(Name<'a>, Description<'a>),
    Warning(Name<'a>, Description<'a>),
    Failed(Name<'a>, Description<'a>),
}

#[derive(Debug, Clone, PartialEq, Copy)]
pub enum HealthStatus {
    Ok,
    Initializing,
    Warning,
    Failed,
}


#[async_trait]
pub trait HealthStatusIndicator<'a>: std::fmt::Debug + Send + Sync {
    fn component_name(&self) -> Name {
        Cow::Borrowed(std::any::type_name::<Self>())
    }

    // TODO(adams): we could have something that checks multiple health points instead of single point.
    async fn check_health(&self) -> Result<HealthStatus, Error>;

    // TODO(adams): internal helper methods or implicits for health status
}


// pub trait HealthStatusIndicator_backup: std::fmt::Debug + Send + Sync + 'static {
//     fn component_name(&self) -> Name {
//         Cow::Borrowed(std::any::type_name::<Self>())
//     }

//     // TODO(adams): we could have something that checks multiple health points instead of single point.
//     fn check_health(&self) -> HealthStatus;

//     // TODO(adams): internal helper methods or implicits for health status
// }

type HealthComponent<'a> = Cow<'a, str>;

#[derive(Debug, Default)]
pub struct HealthRegistry<'a> {
    health_component: HealthComponent<'a>,
    collectors: Vec<Arc<dyn HealthStatusIndicator<'a> + 'a>>,
    dependencies: Vec<HealthRegistry<'a>>,
}

// pub struct HealthRoot {
//     health_registry: HealthRegistry,
// }

impl<'a> HealthRegistry<'a> {
    pub fn new(health_component: HealthComponent<'a>) -> Self {
        Self {
            health_component: health_component,
            ..Default::default()
        }
    }

    // TODO(adams): don't take box, use an Arc, we know all of our stuff are Arcs.
    pub fn register_collector(&mut self, collector: Arc<dyn HealthStatusIndicator<'a> + 'a>) {
        self.collectors.push(collector);
    }

    pub fn add_dependency(&mut self, health_component: HealthComponent<'a>) -> &mut Self {
        // let s    = self.health_component.to_string() + "/" + health_component.to_string().as_str();
        // TODO(adams): pick name at collect.
        let dependency = HealthRegistry {
            health_component: health_component, // NOTE: we could do some name munging here if we wanted.
            ..Default::default()
        };

        self.dependencies.push(dependency);
        self.dependencies
            .last_mut()
            .expect("dependencies should not to be empty.")
    }

    pub fn iter_collectors(&'a self) -> HealthCollectorIterator<'a> {
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

/*

impl HealthRegistry {
    fn flatten_collectors(&self) -> Vec<Box<dyn HealthStatusIndicator>> {
        // Start with the collectors from this registry
        let mut result: Vec<Box<dyn HealthStatusIndicator>> = self.collectors.clone();

        // Recursively flatten collectors from dependent registries
        for dependency in &self.dependencies {
            result.extend_from_slice(&dependency.flatten_collectors());
        }

        result
    }
}
*/

    // pub async fn push_futures2(
    //     mut futures: dyn FuturesUnordered<Pin<Box<dyn futures::Future<Output = std::result::Result<HealthStatus, nativelink_error::Error>> + std::marker::Send>>> ,
    //     collectors: &Vec<Box<dyn HealthStatusIndicator>>,
    //     dependencies: &Vec<HealthRegistry>) {

        // pub async fn push_futures2<Fut: Future<Output = HealthStatus> + Send + Extend<A>>(

    //     pub async fn push_futures2(
    //         mut futures: FuturesUnordered<_>,
    //         collectors: &Vec<Box<dyn HealthStatusIndicator>>,
    //         dependencies: &Vec<HealthRegistry>) {

    //     // futures.extend(collectors.iter().map(|i| {
    //     //     i.check_health()
    //     // }));

    //     for collector in collectors {
    //         futures.push(collector.check_health());
    //     }

    //     for dep in dependencies {
    //         Self::push_futures2(futures, &dep.collectors, &dep.dependencies);
    //     }
    // }

    pub async fn push_futures3(&'a self,

        futures: Arc<Mutex<FuturesUnordered<Pin<Box<dyn futures::Future<Output = std::result::Result<HealthStatus, nativelink_error::Error>> + std::marker::Send + 'a>>>>>,

        collectors: &'a Vec<Arc<dyn HealthStatusIndicator<'a> + 'a>>,

        dependencies: &'a Vec<HealthRegistry<'a>>) {

        // let mut collectors = &*collectors;
        let collectors_box = Box::new(collectors);
        for collector in collectors_box.iter() {
            let c1 = Arc::new(collector);
            let func = move || c1.check_health();
            let func2 = Box::new(func);

            futures.blocking_lock().push( func2()  );
        }

        for dep in dependencies {
            self.push_futures3(futures.clone(), &dep.collectors, &dep.dependencies);
        }


    }

    pub async fn flatten_indicators(&'a self) ->Arc<tokio::sync::Mutex<Vec<HealthStatus>>>
    { // -> Vec<&Result<HealthStatus, Error>> {

    //Result<core::result::Iter<HealthStatus>, Error>{
        use futures::FutureExt;
        use futures::stream::{FuturesUnordered, StreamExt};

        let mut futures = Arc::new(Mutex::new(FuturesUnordered::new())) ;

        // Box::pin(futures);
        // futures.extend(self.collectors.iter().map(|i| {
        //     i.check_health()
        // }));
        // for dep in &self.dependencies {
        //     futures.extend(dep.collectors.iter().map(|i| {
        //         i.check_health()
        //     }));
        // }

        // let mut push_futures
        // = |collectors: &Vec<Box<dyn HealthStatusIndicator>>, dependencies: &Vec<HealthRegistry>| {
        //     futures.extend(collectors.iter().map(|i| {
        //         i.check_health()
        //     }));

        //     for dep in dependencies {
        //         // futures.extend(dep.collectors.iter().map(|i| {
        //         //     i.check_health()
        //         // }));

        //         push_futures(&dep.collectors, &dep.dependencies)
        //     }
        // };


        let c = &self.collectors;
        let d = &self.dependencies;
        let fc = futures.clone();
        self.push_futures3(fc, c, d);


        // let results =
        // while let f = futures.next().await? {
        //     println!("f: {:?}", f);
        // }

        // futures.join_all().await;

        // let ff = futures.collect::<FuturesUnordered<_>>()
        // .try_collect();
        // ff

        // futures.collect()
        // let i = futures.into_iter();
        // let results: Vec<_> = fc.clone().collect().await;
        // let j = futures.clone().collect::<Vec<_>>().await;
        // use futures::stream::{self};
        // let j = futures.collect::<Vec<_>>().await;

        // j
        // let i = futures.await?.into();
        // i

        // let mut v = vec![]; // Vec::new();
        // // let mut fco = futures.into_iter();
        // while let Some(foo) = futures.blocking_lock().next().await {
        //     v.push(foo);
        // };

        // Cow::Owned(v.to_vec())
        // let mut v: Vec<&'a Result<HealthStatus, Error>> = vec![];
        // let mut l = futures.blocking_lock();
        //      while let Some(foo) = l.next().await {
        //     v.push(&foo);
        // };

        // v

        // let mut a = Arc::new(vec![]);
        let a = Arc::new(Mutex::new(vec![]));
        // let b = a.lock().await;
        let mut l = futures.blocking_lock();
        while let Some(foo) = l.next().await {
            match foo {
                Ok(hs) => {
                    a.lock().await.push(hs);
                },
                Err(e) => {
                    println!("e: {:?}", e);
                }
            }
        }
        // b.deref();


        a

    }

    // pub async fn collect_indicators(&self) {
    //     use futures::FutureExt;
    //     use futures::stream::{FuturesUnordered, StreamExt};

    //     let mut futures = FuturesUnordered::new();

    //     self.collectors.iter().map(|c| {
    //         let f = async move {
    //             let check = c.check_health().await?;

    //             HealthStatusDescription {
    //                 health_component: self.health_component,
    //                 health_status: check,
    //             }
    //             // check
    //         };
    //         f
    //     }).for_each(|f| {
    //         futures.push(f);
    //     });

    //     // self.collectors.iter().for_each(|c| {
    //     //     let check = c.check_health();

    //     //     // check.map(|h| {
    //     //     //     // println!("{}: {:?}", c.component_name(), h);
    //     //     //     HealthStatusDescription {
    //     //     //         health_component: c.component_name(),
    //     //     //         health_status: h,
    //     //     //     }
    //     //     // });




    //     //     //println!("{}: {:?}", c.component_name(), h);
    //     // });

    // }

}

pub struct HealthCollectorIterator<'a> {
    health_component: HealthComponent<'a>,

    collector: Option<Box<dyn Iterator<Item = HealthStatusDescription<'a>>>>,
    collectors: std::slice::Iter<'a, Arc<dyn HealthStatusIndicator<'a> + 'a>>,

    dependency_collector_iter: Option<Arc<HealthCollectorIterator<'a>>>,
    dependency_registries: std::slice::Iter<'a, HealthRegistry<'a>>,
}


// impl<'a> Iterator for HealthCollectorIterator<'a> {
//     type Item = HealthStatusDescription;

//     fn next(&mut self) -> Option<Self::Item> {
//         loop {
//             if let Some(m) = self
//             .collector
//             .as_mut()
//             .and_then(|c| c.next())
//             .or_else(|| self.dependency_collector_iter
//                         .as_mut()
//                         .and_then(|i| i.next())
//             ).map(|h| {
//                 Some(h)

//             }) {
//                 return m;
//             }

//             if let Some(collector) = self.collectors.next() {

//                 let h = collector.check_health();

//                 let hc = self.health_component.clone();

//                 let hsd = HealthStatusDescription {
//                     health_component: hc,
//                     health_status: h,
//                 };
//                 self.collector = Some(Box::new(vec![hsd].into_iter()));
//                 continue;
//             }

//             if let Some(collector_iter) = self.dependency_registries.next()
//             .map(|r| Box::new(r.iter_collectors())) {
//                 self.dependency_collector_iter = Some(collector_iter);
//                 continue;
//             }

//             return None;
//         }
//     }
// }

#[derive(Clone, Debug)]
pub struct HealthStatusDescription<'a> {
    health_component: HealthComponent<'a>,
    health_status: HealthStatus,
}
