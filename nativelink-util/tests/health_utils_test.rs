use std::borrow::{BorrowMut, Cow};
use std::sync::Arc;

use async_trait::async_trait;
use nativelink_error::Error;

#[cfg(test)]
mod health_utils_tests {
    use std::borrow::Borrow;

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
        fn check_health(&self) -> Description {
            "no error".into()
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
        let mut health_registery = HealthRegistry::new("nativelink");

        // let mock_component_impl = Arc::new(MockComponentImpl);
        let mock_component_impl = MockComponentImpl;

        // let health_collector = HealthCollector::new(&mock_component_impl);

        // health_registery.register_collector(Box::new(health_collector));

        health_registery.register_collector(Box::new(mock_component_impl));

        let collections = health_registery.iter_collectors();

        println!("{:?}", collections);

        for c in collections {
            println!("{:?}", c);
        }

        Ok(())
    }

    #[tokio::test]
    async fn create_nested() -> Result<(), Error> {
        let mut health_registery = HealthRegistry::new("nativelink");

        // let mock_component_impl = Arc::new(MockComponentImpl);
        let mock_component_impl = MockComponentImpl;

        // let health_collector = HealthCollector::new(&mock_component_impl);

        // health_registery.register_collector(Box::new(health_collector));

        health_registery.register_collector(Box::new(mock_component_impl));

        let mock_component_impl2 = MockComponentImpl;

        let mut nested1 = health_registery.add_dependency("nested1");
        nested1.register_collector(Box::new(mock_component_impl2));

        let mock_component_impl3 = MockComponentImpl;

        let mut nested2 = health_registery.add_dependency("nested2");
        nested2.register_collector(Box::new(mock_component_impl3));

        let mock_component_impl4 = MockComponentImpl;
        nested2.register_collector(Box::new(mock_component_impl4));

        let collections = health_registery.iter_collectors();

        println!("{:?}", collections);

        // Health Status isn't enough, we need to know nesting and/or dependency name
        for c in collections {
            println!("{:?}", c);
        }

        Ok(())
    }
}
