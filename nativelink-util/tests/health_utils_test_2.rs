use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use nativelink_error::Error;

#[cfg(test)]
mod health_utils_tests {
    use nativelink_util::health_utils_2::*;
    use pretty_assertions::assert_eq;

    use super::*;

    #[tokio::test]
    async fn create_empty_indicator() -> Result<(), Error> {
        let health_registery = Arc::new(Mutex::new(HealthRegistry::new("nativelink".into())));

        let health_status = health_registery.lock().unwrap().flatten_indicators().await;
        assert_eq!(health_status.len(), 0);
        //println!("fl: {:?}", fl);
        Ok(())
    }

    #[tokio::test]
    async fn create_register_indicator() -> Result<(), Error> {
        struct MockComponentImpl;
        #[async_trait]
        impl<'a> HealthStatusIndicator<'a> for MockComponentImpl {
            async fn check_health(self: Arc<Self>) -> Result<HealthStatus, Error> {
                Ok(HealthStatus::Ok)
            }
        }

        let health_registery = Arc::new(Mutex::new(HealthRegistry::new("nativelink".into())));

        health_registery
            .lock()
            .unwrap()
            .register_indicator(Arc::new(MockComponentImpl {}));

        let health_status = health_registery.lock().unwrap().flatten_indicators().await;
        assert_eq!(health_status.len(), 1);
        assert_eq!(health_status, vec![HealthStatus::Ok]);
        // println!("fl: {:?}", f1);
        Ok(())
    }
}
