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
    config::ProtocolConfig,
    event::{LatestMilestoneChanged, LatestSolidMilestoneChanged},
    milestone::MilestoneIndex,
    peer::{Peer, PeerManager},
    protocol::ProtocolMetrics,
    tangle::MsTangle,
    worker::{
        BroadcasterWorker, BundleValidatorWorker, HasherWorker, KickstartWorker, MilestoneRequesterWorker,
        MilestoneResponderWorker, MilestoneSolidifierWorker, MilestoneSolidifierWorkerEvent, MilestoneValidatorWorker,
        PeerHandshakerWorker, ProcessorWorker, SolidPropagatorWorker, StatusWorker, StorageWorker, TangleWorker,
        TpsWorker, TransactionRequesterWorker, TransactionResponderWorker,
    },
};

use bee_common_ext::{
    event::Bus,
    node::{Node, NodeBuilder},
};
use bee_crypto::ternary::Hash;
use bee_network::{EndpointId, Network, Origin};
use bee_snapshot::metadata::SnapshotMetadata;
use bee_storage::storage::Backend;

use dashmap::DashMap;
use futures::channel::oneshot;
use log::{debug, error, info};
use tokio::spawn;

use std::{net::SocketAddr, sync::Arc, time::Instant};

static PROTOCOL: spin::RwLock<Option<&'static Protocol>> = spin::RwLock::new(None);

pub struct Protocol {
    pub(crate) network: Network,
    // TODO temporary
    pub(crate) snapshot_timestamp: u64,
    pub(crate) bus: Arc<Bus<'static>>,
    pub(crate) metrics: ProtocolMetrics,
    pub(crate) peer_manager: PeerManager,
    pub(crate) requested_transactions: DashMap<Hash, (MilestoneIndex, Instant)>,
    pub(crate) requested_milestones: DashMap<MilestoneIndex, Instant>,
}

impl Protocol {
    pub fn init<N: Node>(
        config: ProtocolConfig,
        database_config: <N::Backend as Backend>::Config,
        network: Network,
        snapshot_metadata: SnapshotMetadata,
        node_builder: N::Builder,
        bus: Arc<Bus<'static>>,
    ) -> N::Builder {
        let protocol = Protocol {
            network: network.clone(),
            snapshot_timestamp: snapshot_metadata.timestamp(),
            bus,
            metrics: ProtocolMetrics::new(),
            peer_manager: PeerManager::new(),
            requested_transactions: Default::default(),
            requested_milestones: Default::default(),
        };

        *PROTOCOL.write() = Some(Box::leak(Box::new(protocol)));

        let (ms_send, ms_recv) = oneshot::channel();

        node_builder
            .with_worker_cfg::<StorageWorker>(database_config)
            .with_worker_cfg::<TangleWorker>(snapshot_metadata)
            .with_worker_cfg::<HasherWorker>(config.workers.transaction_worker_cache)
            .with_worker_cfg::<ProcessorWorker>(config.clone())
            .with_worker::<TransactionResponderWorker>()
            .with_worker::<MilestoneResponderWorker>()
            .with_worker::<TransactionRequesterWorker>()
            .with_worker::<MilestoneRequesterWorker>()
            .with_worker_cfg::<MilestoneValidatorWorker>(config.clone())
            .with_worker_cfg::<BroadcasterWorker>(network)
            .with_worker::<BundleValidatorWorker>()
            .with_worker::<SolidPropagatorWorker>()
            .with_worker_cfg::<StatusWorker>(config.workers.status_interval)
            .with_worker::<TpsWorker>()
            .with_worker_cfg::<KickstartWorker>((ms_send, config.workers.ms_sync_count))
            .with_worker_cfg::<MilestoneSolidifierWorker>(ms_recv)
    }

    pub fn events<N: Node>(node: &N, config: ProtocolConfig, bus: Arc<Bus<'static>>) {
        let tangle = node.resource::<MsTangle<N::Backend>>();
        bus.add_listener(move |latest_milestone: &LatestMilestoneChanged| {
            info!(
                "New milestone {} {}.",
                *latest_milestone.0.index,
                latest_milestone
                    .0
                    .hash()
                    .iter_trytes()
                    .map(char::from)
                    .collect::<String>()
            );
            tangle.update_latest_milestone_index(latest_milestone.0.index);

            Protocol::broadcast_heartbeat(
                tangle.get_latest_solid_milestone_index(),
                tangle.get_pruning_index(),
                latest_milestone.0.index,
            );
        });

        // bus.add_listener(|latest_solid_milestone: &LatestSolidMilestoneChanged| {
        //     // TODO block_on ?
        //     // TODO uncomment on Chrysalis Pt1.
        //     block_on(Protocol::broadcast_heartbeat(
        //         tangle.get_latest_solid_milestone_index(),
        //         tangle.get_pruning_index(),
        //     ));
        // });

        let milestone_solidifier = node.worker::<MilestoneSolidifierWorker>().unwrap().tx.clone();
        let milestone_requester = node.worker::<MilestoneRequesterWorker>().unwrap().tx.clone();

        let tangle = node.resource::<MsTangle<N::Backend>>();
        bus.add_listener(move |latest_solid_milestone: &LatestSolidMilestoneChanged| {
            debug!("New solid milestone {}.", *latest_solid_milestone.0.index);
            tangle.update_latest_solid_milestone_index(latest_solid_milestone.0.index);

            let ms_sync_count = config.workers.ms_sync_count;
            let next_ms = latest_solid_milestone.0.index + MilestoneIndex(ms_sync_count);

            if tangle.contains_milestone(next_ms) {
                if let Err(e) = milestone_solidifier.send(MilestoneSolidifierWorkerEvent(next_ms)) {
                    error!("Sending solidification event failed: {}", e);
                }
            } else {
                Protocol::request_milestone(&tangle, &milestone_requester, next_ms, None);
            }

            Protocol::broadcast_heartbeat(
                latest_solid_milestone.0.index,
                tangle.get_pruning_index(),
                tangle.get_latest_milestone_index(),
            );
        });
    }

    pub(crate) fn get() -> &'static Protocol {
        *PROTOCOL.read().as_ref().expect("Uninitialized protocol.")
    }

    pub fn register<N: Node>(
        node: &N,
        config: &ProtocolConfig,
        epid: EndpointId,
        address: SocketAddr,
        origin: Origin,
    ) -> (flume::Sender<Vec<u8>>, oneshot::Sender<()>) {
        // TODO check if not already added ?

        let peer = Arc::new(Peer::new(epid, address, origin));

        let (receiver_tx, receiver_rx) = flume::unbounded();
        let (receiver_shutdown_tx, receiver_shutdown_rx) = oneshot::channel();

        Protocol::get().peer_manager.add(peer.clone());

        let tangle = node.resource::<MsTangle<N::Backend>>();
        spawn(
            PeerHandshakerWorker::new(
                Protocol::get().network.clone(),
                config.clone(),
                peer,
                node.worker::<HasherWorker>().unwrap().tx.clone(),
                node.worker::<TransactionResponderWorker>().unwrap().tx.clone(),
                node.worker::<MilestoneResponderWorker>().unwrap().tx.clone(),
                node.worker::<MilestoneRequesterWorker>().unwrap().tx.clone(),
            )
            .run(tangle, receiver_rx, receiver_shutdown_rx),
        );

        (receiver_tx, receiver_shutdown_tx)
    }
}
