#!/bin/bash
# Copyright 2023 The NativeLink Authors. All rights reserved.
#
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#    http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.

# This script may be used to do additonal work to each task before it is launched.
# For example, you may want to possibly download and launch the task into a docker
# container.

export DEVELOPER_DIR="/Applications/Xcode.app/Contents/Developer"
#export XCODE_VERSION_OVERRIDE="13.2.1.13C100"
#export APPLE_SDK_VERSION_OVERRIDE="11.0"
#export APPLE_SDK_PLATFORM="iPhoneOS"
export SDKROOT="/Applications/Xcode.app/Contents/Developer/Platforms/iPhoneSimulator.platform/Developer/SDKs/iPhoneSimulator.sdk"
exec "$@"
