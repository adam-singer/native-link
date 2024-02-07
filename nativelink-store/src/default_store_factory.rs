// Copyright 2023 The NativeLink Authors. All rights reserved.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//    http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::pin::Pin;
use std::sync::Arc;

use futures::stream::FuturesOrdered;
use futures::{Future, TryStreamExt};
use nativelink_config::stores::StoreConfig;
use nativelink_error::Error;
use nativelink_util::health_utils::HealthRegistryBuilder;
use nativelink_util::metrics_utils::Registry;
use nativelink_util::store_trait::Store;

use crate::completeness_checking_store::CompletenessCheckingStore;
use crate::compression_store::CompressionStore;
use crate::dedup_store::DedupStore;
use crate::existence_cache_store::ExistenceCacheStore;
use crate::fast_slow_store::FastSlowStore;
use crate::filesystem_store::FilesystemStore;
use crate::grpc_store::GrpcStore;
use crate::memory_store::MemoryStore;
use crate::noop_store::NoopStore;
use crate::ref_store::RefStore;
use crate::s3_store::S3Store;
use crate::shard_store::ShardStore;
use crate::size_partitioning_store::SizePartitioningStore;
use crate::store_manager::StoreManager;
use crate::verify_store::VerifyStore;

type FutureMaybeStore<'a> = Box<dyn Future<Output = Result<Arc<dyn Store>, Error>> + 'a>;

pub fn store_factory<'a>(
    backend: &'a StoreConfig,
    store_manager: &'a Arc<StoreManager>,
    maybe_store_metrics: Option<&'a mut Registry>,
    maybe_health_registry_builder: Option<&'a mut HealthRegistryBuilder>,
) -> Pin<FutureMaybeStore<'a>> {
    Box::pin(async move {
        let store: Arc<dyn Store> = match backend {
            StoreConfig::memory(config) => {
                let store = Arc::new(MemoryStore::new(config));

                if let Some(health_registry_builder) = maybe_health_registry_builder {
                    store.clone().register_health(health_registry_builder);
                }
                store
            }

            StoreConfig::experimental_s3_store(config) => {
                let store = Arc::new(S3Store::new(config).await?);

                if let Some(health_registry_builder) = maybe_health_registry_builder {
                    store.clone().register_health(health_registry_builder);
                }
                store
            }

            StoreConfig::verify(config) => {
                let health_registry_builder = maybe_health_registry_builder.unwrap();
                let mut verify_health_registry = health_registry_builder.sub_builder("verify_store".into());
                let mut inner_store_health_registry = health_registry_builder.sub_builder("inner_store".into());

                let inner_store = store_factory(
                    &config.backend,
                    store_manager,
                    None,
                    Some(&mut inner_store_health_registry),
                )
                .await?;

                let store = Arc::new(VerifyStore::new(config, inner_store));

                if let Some(health_registry_builder) = Some(&mut verify_health_registry) {
                    store.clone().register_health(health_registry_builder);
                }

                store
            }

            StoreConfig::compression(config) => {
                let health_registry_builder = maybe_health_registry_builder.unwrap();
                let mut compression_health_registry = health_registry_builder.sub_builder("compression".into());
                let mut inner_store_health_registry = health_registry_builder.sub_builder("inner_store".into());

                let inner_store = store_factory(
                    &config.backend,
                    store_manager,
                    None,
                    Some(&mut inner_store_health_registry),
                )
                .await?;

                let store = Arc::new(CompressionStore::new(*config.clone(), inner_store)?);

                if let Some(health_registry_builder) = Some(&mut compression_health_registry) {
                    store.clone().register_health(health_registry_builder);
                }

                store
            }

            StoreConfig::dedup(config) => {
                let health_registry_builder = maybe_health_registry_builder.unwrap();
                let mut dedup_health_registry = health_registry_builder.sub_builder("dedup".into());
                let mut index_store_health_registry = health_registry_builder.sub_builder("index_store".into());
                let mut content_store_registry = health_registry_builder.sub_builder("content_store".into());

                let index_store = store_factory(
                    &config.index_store,
                    store_manager,
                    None,
                    Some(&mut index_store_health_registry),
                )
                .await?;
                let content_store = store_factory(
                    &config.content_store,
                    store_manager,
                    None,
                    Some(&mut content_store_registry),
                )
                .await?;

                let store = Arc::new(DedupStore::new(config, index_store, content_store));

                if let Some(health_registry_builder) = Some(&mut dedup_health_registry) {
                    store.clone().register_health(health_registry_builder);
                }

                store
            }

            StoreConfig::existence_cache(config) => {
                let health_registry_builder = maybe_health_registry_builder.unwrap();
                let mut existence_cache_health_registry = health_registry_builder.sub_builder("existence_cache".into());
                let mut inner_store_health_registry = health_registry_builder.sub_builder("inner_store".into());

                let inner_store = store_factory(
                    &config.backend,
                    store_manager,
                    None,
                    Some(&mut inner_store_health_registry),
                )
                .await?;

                let store = Arc::new(ExistenceCacheStore::new(config, inner_store));

                if let Some(health_registry_builder) = Some(&mut existence_cache_health_registry) {
                    store.clone().register_health(health_registry_builder);
                }

                store
            }

            StoreConfig::completeness_checking(config) => {
                let health_registry_builder = maybe_health_registry_builder.unwrap();

                let mut completeness_checking_health_registry =
                    health_registry_builder.sub_builder("completeness_checking".into());
                let mut ac_health_registry = health_registry_builder.sub_builder("ac".into());
                let mut cas_health_registry = health_registry_builder.sub_builder("cas".into());

                let ac_store =
                    store_factory(&config.backend, store_manager, None, Some(&mut ac_health_registry)).await?;
                let cas_store =
                    store_factory(&config.cas_store, store_manager, None, Some(&mut cas_health_registry)).await?;

                let store = Arc::new(CompletenessCheckingStore::new(ac_store, cas_store));

                if let Some(health_registry_builder) = Some(&mut completeness_checking_health_registry) {
                    store.clone().register_health(health_registry_builder);
                }

                store
            }

            StoreConfig::fast_slow(config) => {
                let health_registry_builder = maybe_health_registry_builder.unwrap();

                let mut fast_slow_store_health_registry = health_registry_builder.sub_builder("fast_slow".into());
                let mut fast_health_registry = health_registry_builder.sub_builder("fast".into());
                let mut slow_health_registry = health_registry_builder.sub_builder("slow".into());

                let fast_store =
                    store_factory(&config.fast, store_manager, None, Some(&mut fast_health_registry)).await?;
                let slow_store =
                    store_factory(&config.slow, store_manager, None, Some(&mut slow_health_registry)).await?;

                let store = Arc::new(FastSlowStore::new(config, fast_store, slow_store));

                if let Some(health_registry_builder) = Some(&mut fast_slow_store_health_registry) {
                    store.clone().register_health(health_registry_builder);
                }

                store
            }

            StoreConfig::filesystem(config) => {
                let store = Arc::new(<FilesystemStore>::new(config).await?);

                if let Some(health_registry_builder) = maybe_health_registry_builder {
                    store.clone().register_health(health_registry_builder);
                }

                store
            }

            StoreConfig::ref_store(config) => {
                let store = Arc::new(RefStore::new(config, Arc::downgrade(store_manager)));

                if let Some(health_registry_builder) = maybe_health_registry_builder {
                    store.clone().register_health(health_registry_builder);
                }

                store
            }

            StoreConfig::size_partitioning(config) => {
                let health_registry_builder = maybe_health_registry_builder.unwrap();

                let mut size_partitioning_store_health_registry =
                    health_registry_builder.sub_builder("size_partitioning".into());
                let mut lower_health_registry = health_registry_builder.sub_builder("lower".into());
                let mut upper_health_registry = health_registry_builder.sub_builder("upper".into());

                let lower_store = store_factory(
                    &config.lower_store,
                    store_manager,
                    None,
                    Some(&mut lower_health_registry),
                )
                .await?;
                let upper_store = store_factory(
                    &config.upper_store,
                    store_manager,
                    None,
                    Some(&mut upper_health_registry),
                )
                .await?;

                let store = Arc::new(SizePartitioningStore::new(config, lower_store, upper_store));

                if let Some(health_registry_builder) = Some(&mut size_partitioning_store_health_registry) {
                    store.clone().register_health(health_registry_builder);
                }

                store
            }

            StoreConfig::grpc(config) => {
                let store = Arc::new(GrpcStore::new(config).await?);

                if let Some(health_registry_builder) = maybe_health_registry_builder {
                    store.clone().register_health(health_registry_builder);
                }

                store
            }

            StoreConfig::noop => {
                let store = Arc::new(NoopStore::new());

                if let Some(health_registry_builder) = maybe_health_registry_builder {
                    store.clone().register_health(health_registry_builder);
                }

                store
            }

            StoreConfig::shard(config) => {
                // TODO(adams): need to fix up how scoping works here for health checks.
                let health_registry_builder = maybe_health_registry_builder.unwrap();

                let mut shard_store_health_registry = health_registry_builder.sub_builder("shard_store".into());
                // //let mut lower_health_registry = health_registry_builder.sub_builder("lower".into());
                // let stores_len = config.stores.len();
                // let mut shards_health_registries: Vec<&mut HealthRegistryBuilder> = Vec::new();
                // for _ in 0..stores_len {
                //     shards_health_registries.push(&mut health_registry_builder.sub_builder("shard".into()));
                // }

                let stores = config
                    .stores
                    .iter()
                    .map(|store_config| {
                        //let shard_health_registry = *shards_health_registries.index(0); /// health_registry_builder.sub_builder("shard".into());

                        let store = store_factory(&store_config.store, store_manager, None, None);

                        store
                    })
                    .collect::<FuturesOrdered<_>>()
                    .try_collect::<Vec<_>>()
                    .await?;
                let store = Arc::new(ShardStore::new(config, stores)?);

                if let Some(health_registry_builder) = Some(&mut shard_store_health_registry) {
                    store.clone().register_health(health_registry_builder);
                }

                store
            }
        };
        if let Some(store_metrics) = maybe_store_metrics {
            store.clone().register_metrics(store_metrics);
        }

        Ok(store)
    })
}
