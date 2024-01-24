use std::borrow::{BorrowMut, Cow};
use std::sync::Arc;

use async_trait::async_trait;
use nativelink_error::Error;

#[cfg(test)]
mod health_utils_tests {
    use std::borrow::Borrow;

    use futures::StreamExt;
    // use alloc::collections;
    use nativelink_util::health_utils::*;
    use pretty_assertions::assert_eq;

    use super::*; // Must be declared in every module.

    #[async_trait]
    pub trait MockComponent: Sync + Send + Unpin + HealthStatusIndicator {}

    #[derive(Debug)]
    struct MockComponentImpl;

    impl MockComponent for MockComponentImpl {
        // TODO: Most likely this will need to be async
        // fn check_health(&self) -> Description {
        //     "no error".into()
        // }
    }

    impl HealthStatusIndicator for MockComponentImpl {
        fn check_health(&self) -> HealthStatus {
            HealthStatus::Ok(self.component_name(), "no error".into())
        }
    }

    // impl HealthCollectorTrait for MockComponentImpl {
    //     fn collect<'a>(&'a self) -> Box<dyn Iterator<Item = std::borrow::Cow<'a, HealthStatus>> + 'a> {
    //         // Box::new(vec![Cow::Borrowed(self.internal_check_health())].into_iter())
    //         Box::new(vec![Cow::Borrowed(&self.internal_check_health())].into_iter())
    //     }
    // }

    #[tokio::test]
    async fn create_registery() -> Result<(), Error> {
        let mut health_registery = HealthRegistry::new("nativelink".into());

        // let mock_component_impl = Arc::new(MockComponentImpl);
        let mock_component_impl = MockComponentImpl;

        // let health_collector = HealthCollector::new(&mock_component_impl);

        // health_registery.register_collector(Box::new(health_collector));

        health_registery.register_collector(Box::new(mock_component_impl));

        let collections = health_registery.iter_collectors();

        // println!("{:?}", collections);

        for c in collections {
            println!("{:?}", c);
        }

        Ok(())
    }

    #[tokio::test]
    async fn create_nested() -> Result<(), Error> {
        let mut health_registery = HealthRegistry::new("nativelink".into());

        // let mock_component_impl = Arc::new(MockComponentImpl);
        let mock_component_impl = MockComponentImpl;

        // let health_collector = HealthCollector::new(&mock_component_impl);

        // health_registery.register_collector(Box::new(health_collector));

        health_registery.register_collector(Box::new(mock_component_impl));

        let mock_component_impl2 = MockComponentImpl;

        let mut nested1 = health_registery.add_dependency("nested1".into());
        nested1.register_collector(Box::new(mock_component_impl2));

        let mock_component_impl3 = MockComponentImpl;

        let mut nested2 = health_registery.add_dependency("nested2".into());
        nested2.register_collector(Box::new(mock_component_impl3));

        let mock_component_impl4 = MockComponentImpl;
        nested2.register_collector(Box::new(mock_component_impl4));

        let nested5 = nested2.add_dependency("nested5".into());
        let mock_component_impl5 = MockComponentImpl;
        nested5.register_collector(Box::new(mock_component_impl5));

        let mock_component_impl6 = MockComponentImpl;
        nested5.register_collector(Box::new(mock_component_impl6));

        let collections = health_registery.iter_collectors();

        // println!("{:?}", collections);

        // Health Status isn't enough, we need to know nesting and/or dependency name
        for c in collections {
            println!("{:?}", c);
        }

        Ok(())
    }

    #[tokio::test]
    async fn create_nested_1() -> Result<(), Error> {
        use std::time::{Duration, Instant};

        use async_stream::stream;
        use tokio::time::MissedTickBehavior::Delay;
        // use async_stream::stream;

        // use futures_core::stream::Stream;
        // use futures_util::pin_mut;
        // use futures_util::stream::StreamExt;

        // let s = stream! {
        //     let mut when = Instant::now();
        //     for _ in 0..3 {
        //         // let delay = Delay { when };
        //         // delay.await;
        //         yield ();
        //         when += Duration::from_millis(10);
        //     }
        // };

        // let i = s.try_collect::<Vec<_>>().await?;
        // s.for_each(|_| async {
        //     println!("Hello world!");
        // }).await;
        // pin_mut!(s); // needed for iteration

        // while let Some(value) = s.next().await {
        //     println!("got {}", value);
        // }

        let mut o = Some(Some("hello"));
        println!("o = {:?}", o);
        let o2 = o.take();
        println!("o2 = {:?}", o2);
        println!("o = {:?}", o);
        let k: Option<Result<Option<&str>, Error>> = o2.map(Ok);
        println!("k = {:?}", k);

        Ok(())
    }
}
