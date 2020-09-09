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

use crate::storage::Backend;

pub trait Persistable<S: Backend> {
    /// This encode method will extend the provided buffer and return ();
    fn encode_persistable(&self, buffer: &mut Vec<u8>);
    /// Decode `slice` and return Self
    fn decode_persistable(slice: &[u8]) -> Self
    where
        Self: Sized;
}