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
    milestone::MilestoneIndex,
    protocol::Protocol,
    tangle::MsTangle,
    worker::{TangleWorker, TransactionRequesterWorker, TransactionRequesterWorkerEvent},
};

use bee_common::{shutdown_stream::ShutdownStream, worker::Error as WorkerError};
use bee_common_ext::{node::Node, worker::Worker};
use bee_storage::storage::Backend;
use bee_tangle::traversal;

use async_trait::async_trait;
use futures::{channel::oneshot, StreamExt};
use log::{debug, info};

use std::any::TypeId;

pub(crate) struct MilestoneSolidifierWorkerEvent(pub MilestoneIndex);

pub(crate) struct MilestoneSolidifierWorker {
    pub(crate) tx: flume::Sender<MilestoneSolidifierWorkerEvent>,
}

async fn trigger_solidification_unchecked<B: Backend>(
    tangle: &MsTangle<B>,
    transaction_requester: &flume::Sender<TransactionRequesterWorkerEvent>,
    target_index: MilestoneIndex,
    next_ms_index: &mut MilestoneIndex,
) {
    if let Some(target_hash) = tangle.get_milestone_hash(target_index) {
        if !tangle.is_solid_transaction(&target_hash) {
            debug!("Triggered solidification for milestone {}.", *target_index);

            // TODO: This wouldn't be necessary if the traversal code wasn't closure-driven
            let mut missing = Vec::new();

            traversal::visit_parents_depth_first(
                &**tangle,
                target_hash,
                |hash, _, metadata| {
                    (!metadata.flags().is_requested() || *hash == target_hash)
                        && !metadata.flags().is_solid()
                        && !Protocol::get().requested_transactions.contains_key(&hash)
                },
                |_, _, _| {},
                |_, _, _| {},
                |missing_hash| missing.push(*missing_hash),
            );

            for missing_hash in missing {
                Protocol::request_transaction(tangle, transaction_requester, missing_hash, target_index).await;
            }

            *next_ms_index = target_index + MilestoneIndex(1);
        }
    }
}

fn save_index(target_index: MilestoneIndex, queue: &mut Vec<MilestoneIndex>) {
    debug!("Storing milestone {}.", *target_index);
    if let Err(pos) = queue.binary_search_by(|index| target_index.cmp(index)) {
        queue.insert(pos, target_index);
    }
}

#[async_trait]
impl<N: Node> Worker<N> for MilestoneSolidifierWorker {
    type Config = oneshot::Receiver<MilestoneIndex>;
    type Error = WorkerError;

    fn dependencies() -> &'static [TypeId] {
        Box::leak(Box::from(vec![
            TypeId::of::<TransactionRequesterWorker>(),
            TypeId::of::<TangleWorker>(),
        ]))
    }

    async fn start(node: &mut N, config: Self::Config) -> Result<Self, Self::Error> {
        let (tx, rx) = flume::unbounded();
        let transaction_requester = node.worker::<TransactionRequesterWorker>().unwrap().tx.clone();

        let tangle = node.resource::<MsTangle<N::Backend>>();

        node.spawn::<Self, _, _>(|shutdown| async move {
            info!("Running.");

            let mut receiver = ShutdownStream::new(shutdown, rx.into_stream());

            let mut queue = vec![];
            let mut next_ms_index = config.await.unwrap();

            while let Some(MilestoneSolidifierWorkerEvent(index)) = receiver.next().await {
                save_index(index, &mut queue);
                while let Some(index) = queue.pop() {
                    if index == next_ms_index {
                        trigger_solidification_unchecked(&tangle, &transaction_requester, index, &mut next_ms_index)
                            .await;
                    } else {
                        queue.push(index);
                        break;
                    }
                }
            }

            info!("Stopped.");
        });

        Ok(Self { tx })
    }
}
