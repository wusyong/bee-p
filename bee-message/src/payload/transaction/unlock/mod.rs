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

mod reference;
mod signature;

pub use reference::ReferenceUnlock;
pub use signature::{Ed25519Signature, SignatureUnlock, WotsSignature};

use bee_common_ext::packable::{Error as PackableError, Packable, Read, Write};

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub enum UnlockBlock {
    Signature(SignatureUnlock),
    Reference(ReferenceUnlock),
}

impl From<SignatureUnlock> for UnlockBlock {
    fn from(signature: SignatureUnlock) -> Self {
        Self::Signature(signature)
    }
}

impl From<ReferenceUnlock> for UnlockBlock {
    fn from(reference: ReferenceUnlock) -> Self {
        Self::Reference(reference)
    }
}

impl Packable for UnlockBlock {
    fn packed_len(&self) -> usize {
        match self {
            Self::Signature(unlock) => 0u8.packed_len() + unlock.packed_len(),
            Self::Reference(unlock) => 1u8.packed_len() + unlock.packed_len(),
        }
    }

    fn pack<W: Write>(&self, buf: &mut W) -> Result<(), PackableError> {
        match self {
            Self::Signature(unlock) => {
                0u8.pack(buf)?;
                unlock.pack(buf)?;
            }
            Self::Reference(unlock) => {
                1u8.pack(buf)?;
                unlock.pack(buf)?;
            }
        }

        Ok(())
    }

    fn unpack<R: Read>(buf: &mut R) -> Result<Self, PackableError>
    where
        Self: Sized,
    {
        Ok(match u8::unpack(buf)? {
            0 => Self::Signature(SignatureUnlock::unpack(buf)?),
            1 => Self::Reference(ReferenceUnlock::unpack(buf)?),
            _ => return Err(PackableError::InvalidVariant),
        })
    }
}
