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

mod hash;
mod message;

pub mod payload;

pub use hash::Hash;
pub use message::{Message, MessageBuilder};

use core::fmt;

#[derive(Debug)]
pub enum Error {
    AmountError,
    CountError,
    EmptyError,
    DuplicateError,
    IndexError,
    OrderError,
    HashError,
    PathError,
    MissingParameter,
    BincodeError(bincode::Error),
    SigningError(bee_signing_ext::binary::Error),
    SignatureError(bee_signing_ext::SignatureError),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::AmountError => write!(f, "Invalid amount provided."),
            Error::CountError => write!(f, "Invalid count number provided."),
            Error::EmptyError => write!(f, "The length of the object is empty."),
            Error::DuplicateError => write!(f, "The object in the set must be unique."),
            Error::IndexError => write!(f, "The position of index is not correct."),
            Error::OrderError => write!(f, "The vector is not sorted by lexicographical order."),
            Error::HashError => write!(f, "The format of provided hash is not correct."),
            Error::PathError => write!(f, "The format of provided BIP32 path is not correct."),
            Error::MissingParameter => write!(f, "Missing required parameters."),
            Error::BincodeError(e) => write!(f, "{}", e),
            Error::SigningError(e) => write!(f, "{}", e),
            Error::SignatureError(e) => write!(f, "{}", e),
        }
    }
}

impl From<bincode::Error> for Error {
    fn from(error: bincode::Error) -> Self {
        Error::BincodeError(error)
    }
}

impl From<bee_signing_ext::binary::Error> for Error {
    fn from(error: bee_signing_ext::binary::Error) -> Self {
        Error::SigningError(error)
    }
}

impl From<bee_signing_ext::SignatureError> for Error {
    fn from(error: bee_signing_ext::SignatureError) -> Self {
        Error::SignatureError(error)
    }
}
