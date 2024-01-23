use std::borrow::{BorrowMut, Cow};
use std::sync::Arc;

use async_trait::async_trait;
use nativelink_error::Error;

#[cfg(test)]
mod health_utils_tests {
    use std::borrow::Borrow;

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

    impl HealthCollectorTrait for MockComponentImpl {
        fn collect<'a>(&'a self) -> Box<dyn Iterator<Item = std::borrow::Cow<'a, HealthStatus>> + 'a> {
            // Box::new(vec![Cow::Borrowed(self.internal_check_health())].into_iter())
            Box::new(vec![Cow::Borrowed(&self.internal_check_health())].into_iter())
        }
    }

    #[tokio::test]
    async fn create_registery() -> Result<(), Error> {
        let health_registery = HealthRegistry::new("nativelink");

        let mock_component_impl = Arc::new(MockComponentImpl {});

        let health_collector = HealthCollector::new(&mock_component_impl);

        health_registery.register_collector(Box::new(health_collector));

        Ok(())
    }
}
