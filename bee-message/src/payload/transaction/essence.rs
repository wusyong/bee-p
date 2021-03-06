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

use crate::{
    payload::{
        transaction::{input::Input, output::Output},
        Payload,
    },
    Error,
};

use bee_common_ext::packable::{Error as PackableError, Packable, Read, Write};

use serde::{Deserialize, Serialize};

use alloc::vec::Vec;

// TODO remove pub(crate)
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TransactionEssence {
    pub(crate) inputs: Box<[Input]>,
    pub(crate) outputs: Box<[Output]>,
    pub(crate) payload: Option<Payload>,
}

impl TransactionEssence {
    pub fn builder() -> TransactionEssenceBuilder {
        TransactionEssenceBuilder::new()
    }

    pub fn inputs(&self) -> &[Input] {
        &self.inputs
    }

    pub fn outputs(&self) -> &[Output] {
        &self.outputs
    }

    pub fn payload(&self) -> &Option<Payload> {
        &self.payload
    }
}

impl Packable for TransactionEssence {
    fn packed_len(&self) -> usize {
        0u8.packed_len()
            + 0u16.packed_len()
            + self.inputs.iter().map(|input| input.packed_len()).sum::<usize>()
            + 0u16.packed_len()
            + self.outputs.iter().map(|output| output.packed_len()).sum::<usize>()
            + 0u32.packed_len()
            + self.payload.iter().map(|payload| payload.packed_len()).sum::<usize>()
    }

    fn pack<W: Write>(&self, buf: &mut W) -> Result<(), PackableError> {
        0u8.pack(buf)?;

        (self.inputs.len() as u16).pack(buf)?;
        for input in self.inputs.iter() {
            input.pack(buf)?;
        }

        (self.outputs.len() as u16).pack(buf)?;
        for output in self.outputs.iter() {
            output.pack(buf)?;
        }

        match self.payload {
            Some(ref payload) => {
                (payload.packed_len() as u32).pack(buf)?;
                payload.pack(buf)?;
            }
            None => 0u32.pack(buf)?,
        }

        Ok(())
    }

    fn unpack<R: Read>(buf: &mut R) -> Result<Self, PackableError>
    where
        Self: Sized,
    {
        if u8::unpack(buf)? != 0u8 {
            return Err(PackableError::InvalidType);
        }

        let inputs_len = u16::unpack(buf)? as usize;
        let mut inputs = Vec::with_capacity(inputs_len);
        for _ in 0..inputs_len {
            inputs.push(Input::unpack(buf)?);
        }

        let outputs_len = u16::unpack(buf)? as usize;
        let mut outputs = Vec::with_capacity(outputs_len);
        for _ in 0..outputs_len {
            outputs.push(Output::unpack(buf)?);
        }

        let payload_len = u32::unpack(buf)? as usize;
        let payload = if payload_len > 0 {
            let payload = Payload::unpack(buf)?;
            if payload_len != payload.packed_len() {
                return Err(PackableError::InvalidAnnouncedLen);
            }

            Some(payload)
        } else {
            None
        };

        Ok(Self {
            inputs: inputs.into_boxed_slice(),
            outputs: outputs.into_boxed_slice(),
            payload,
        })
    }
}

#[derive(Debug, Default)]
pub struct TransactionEssenceBuilder {
    inputs: Vec<Input>,
    outputs: Vec<Output>,
    payload: Option<Payload>,
}

impl TransactionEssenceBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_input(mut self, input: Input) -> Self {
        self.inputs.push(input);
        self
    }

    pub fn add_output(mut self, output: Output) -> Self {
        self.outputs.push(output);
        self
    }

    pub fn with_payload(mut self, payload: Payload) -> Self {
        self.payload = Some(payload);
        self
    }

    pub fn finish(self) -> Result<TransactionEssence, Error> {
        if self.inputs.is_empty() {
            return Err(Error::NoInput);
        }

        if self.outputs.is_empty() {
            return Err(Error::NoOutput);
        }

        Ok(TransactionEssence {
            inputs: self.inputs.into_boxed_slice(),
            outputs: self.outputs.into_boxed_slice(),
            payload: self.payload,
        })
    }
}
