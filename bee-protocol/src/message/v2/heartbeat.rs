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

//! Heartbeat message of the protocol version 2

// TODO comment/uncomment when Chrysalis Pt1 is released.

use crate::message::Message;

use std::{convert::TryInto, ops::Range};

const LATEST_SOLID_MILESTONE_INDEX_SIZE: usize = 4;
const PRUNED_INDEX_SIZE: usize = 4;
const LATEST_MILESTONE_INDEX_SIZE: usize = 4;
const CONNECTED_PEERS_SIZE: usize = 1;
const SYNCED_PEERS_SIZE: usize = 1;
const CONSTANT_SIZE: usize = LATEST_SOLID_MILESTONE_INDEX_SIZE
    + PRUNED_INDEX_SIZE
    + LATEST_MILESTONE_INDEX_SIZE
    + CONNECTED_PEERS_SIZE
    + SYNCED_PEERS_SIZE;

/// A message that informs about the part of the tangle currently being fully stored by a node.
/// This message is sent when a node:
/// - just got paired to another node;
/// - did a local snapshot and pruned away a part of the tangle;
/// - solidified a new milestone;
/// It also helps other nodes to know if they can ask it a specific transaction.
#[derive(Default)]
pub(crate) struct Heartbeat {
    /// Index of the latest solid milestone.
    pub(crate) latest_solid_milestone_index: u32,
    /// Pruned index.
    pub(crate) pruned_index: u32,
    /// Index of the latest milestone.
    pub(crate) latest_milestone_index: u32,
    /// Number of connected peers.
    pub(crate) connected_peers: u8,
    /// Number of synced peers.
    pub(crate) synced_peers: u8,
}

impl Heartbeat {
    pub(crate) fn new(
        latest_solid_milestone_index: u32,
        pruned_index: u32,
        latest_milestone_index: u32,
        connected_peers: u8,
        synced_peers: u8,
    ) -> Self {
        Self {
            latest_solid_milestone_index,
            pruned_index,
            latest_milestone_index,
            connected_peers,
            synced_peers,
        }
    }
}

impl Message for Heartbeat {
    const ID: u8 = 0x06;

    fn size_range() -> Range<usize> {
        (CONSTANT_SIZE)..(CONSTANT_SIZE + 1)
    }

    fn from_bytes(bytes: &[u8]) -> Self {
        let mut message = Self::default();

        let (bytes, next) = bytes.split_at(LATEST_SOLID_MILESTONE_INDEX_SIZE);
        message.latest_solid_milestone_index = u32::from_be_bytes(bytes.try_into().expect("Invalid buffer size"));

        let (bytes, next) = next.split_at(PRUNED_INDEX_SIZE);
        message.pruned_index = u32::from_be_bytes(bytes.try_into().expect("Invalid buffer size"));

        let (bytes, next) = next.split_at(LATEST_MILESTONE_INDEX_SIZE);
        message.latest_milestone_index = u32::from_be_bytes(bytes.try_into().expect("Invalid buffer size"));

        let (bytes, next) = next.split_at(CONNECTED_PEERS_SIZE);
        message.connected_peers = u8::from_be_bytes(bytes.try_into().expect("Invalid buffer size"));

        let (bytes, _) = next.split_at(SYNCED_PEERS_SIZE);
        message.synced_peers = u8::from_be_bytes(bytes.try_into().expect("Invalid buffer size"));

        message
    }

    fn size(&self) -> usize {
        CONSTANT_SIZE
    }

    fn into_bytes(self, bytes: &mut [u8]) {
        let (bytes, next) = bytes.split_at_mut(LATEST_SOLID_MILESTONE_INDEX_SIZE);
        bytes.copy_from_slice(&self.latest_solid_milestone_index.to_be_bytes());
        let (bytes, next) = next.split_at_mut(PRUNED_INDEX_SIZE);
        bytes.copy_from_slice(&self.pruned_index.to_be_bytes());
        let (bytes, next) = next.split_at_mut(LATEST_MILESTONE_INDEX_SIZE);
        bytes.copy_from_slice(&self.latest_milestone_index.to_be_bytes());
        let (bytes, next) = next.split_at_mut(CONNECTED_PEERS_SIZE);
        bytes.copy_from_slice(&self.connected_peers.to_be_bytes());
        let (bytes, _) = next.split_at_mut(SYNCED_PEERS_SIZE);
        bytes.copy_from_slice(&self.synced_peers.to_be_bytes());
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    const LATEST_SOLID_MILESTONE_INDEX: u32 = 0x0118_1f9b;
    const PRUNED_INDEX: u32 = 0x3dc2_97b4;
    const LATEST_MILESTONE_INDEX: u32 = 0x60be_20c2;
    const CONNECTED_PEERS: u8 = 12;
    const SYNCED_PEERS: u8 = 5;

    #[test]
    fn id() {
        assert_eq!(Heartbeat::ID, 6);
    }

    #[test]
    fn size_range() {
        assert_eq!(Heartbeat::size_range().contains(&13), false);
        assert_eq!(Heartbeat::size_range().contains(&14), true);
        assert_eq!(Heartbeat::size_range().contains(&15), false);
    }

    #[test]
    fn size() {
        let message = Heartbeat::new(
            LATEST_SOLID_MILESTONE_INDEX,
            PRUNED_INDEX,
            LATEST_MILESTONE_INDEX,
            CONNECTED_PEERS,
            SYNCED_PEERS,
        );

        assert_eq!(message.size(), CONSTANT_SIZE);
    }

    #[test]
    fn into_from() {
        let message_from = Heartbeat::new(
            LATEST_SOLID_MILESTONE_INDEX,
            PRUNED_INDEX,
            LATEST_MILESTONE_INDEX,
            CONNECTED_PEERS,
            SYNCED_PEERS,
        );
        let mut bytes = vec![0u8; message_from.size()];
        message_from.into_bytes(&mut bytes);
        let message_to = Heartbeat::from_bytes(&bytes);

        assert_eq!(message_to.latest_solid_milestone_index, LATEST_SOLID_MILESTONE_INDEX);
        assert_eq!(message_to.pruned_index, PRUNED_INDEX);
        assert_eq!(message_to.latest_milestone_index, LATEST_MILESTONE_INDEX);
        assert_eq!(message_to.connected_peers, CONNECTED_PEERS);
        assert_eq!(message_to.synced_peers, SYNCED_PEERS);
    }
}
