// Copyright 2020 IOTA Stiftung
//
// Licensed under the Apache License, Version 2.0 (the "License"); you may not use this file except in compliance with
// the License. You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software distributed under the License is distributed on
// an "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and limitations under the License.

pub mod batch;
pub mod delete;
pub mod fetch;
pub mod insert;

use bee_storage::access::Error;

#[derive(Debug)]
pub struct OpError {
    is_retryable: bool,
    is_still_valid: bool,
    error_msg: Option<String>,
}

impl Error for OpError {
    fn is_retryable(&self) -> bool {
        self.is_retryable
    }
    fn is_still_valid(&self) -> bool {
        self.is_still_valid
    }
    fn error_msg(&self) -> Option<String> {
        self.error_msg.clone()
    }
}

impl From<::rocksdb::Error> for OpError {
    fn from(err: ::rocksdb::Error) -> Self {
        Self {
            is_retryable: false,
            is_still_valid: false,
            error_msg: Some(err.into_string()),
        }
    }
}
