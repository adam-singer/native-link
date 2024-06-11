// Copyright 2024 The NativeLink Authors. All rights reserved.
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

use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::watch::Receiver;

use nativelink_error::Error;
use nativelink_util::action_messages::{ActionInfo, ActionState};

use crate::operation_state_manager::ActionStateResult;

pub(crate) struct ClientActionStateResult {
    //notify_channel: Receiver<Arc<ActionState>>,
    rx: Receiver<Arc<ActionState>>,
}

impl ClientActionStateResult {
    pub(crate) fn new(mut rx: Receiver<Arc<ActionState>>) -> Self {
        rx.mark_changed();
        Self { rx }
    }
}

#[async_trait]
impl ActionStateResult for ClientActionStateResult {
    async fn as_state(&self) -> Result<Arc<ActionState>, Error> {
        Ok(self.rx.borrow().clone())
    }

    async fn as_receiver(&self) -> Result<&'_ Receiver<Arc<ActionState>>, Error> {
        Ok(&self.rx)
    }

    async fn as_action_info(&self) -> Result<Arc<ActionInfo>, Error> {
        unimplemented!()
    }
}
