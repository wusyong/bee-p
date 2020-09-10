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

use crate::{milestone::MilestoneIndex, protocol::Protocol, tangle::tangle, worker::MilestoneSolidifierWorkerEvent};

use bee_common::worker::Error as WorkerError;

use futures::{channel::oneshot, future::Fuse, select, FutureExt};
use log::info;

pub(crate) const MS_BATCH_SIZE: u32 = 5;

pub(crate) struct KickstartWorker {
    shutdown: Fuse<oneshot::Receiver<()>>,
}

impl KickstartWorker {
    pub(crate) fn new(shutdown: oneshot::Receiver<()>) -> Self {
        Self {
            shutdown: shutdown.fuse(),
        }
    }

    pub(crate) async fn run(mut self) -> Result<(), WorkerError> {
        info!("Running.");

        loop {
            async_std::task::sleep(std::time::Duration::from_secs(1)).await;
            select! {
                _ = &mut self.shutdown => break,
                default => {
                    let next_ms = *tangle().get_latest_solid_milestone_index() + 1;
                    let latest_ms = *tangle().get_latest_milestone_index();

                    if Protocol::get().peer_manager.handshaked_peers.len() != 0 && next_ms + MS_BATCH_SIZE < latest_ms {
                        Protocol::get().milestone_solidifier_worker.unbounded_send(MilestoneSolidifierWorkerEvent::SetNextMilestone(MilestoneIndex(next_ms)));

                        for index in next_ms..(next_ms + MS_BATCH_SIZE) {
                            Protocol::request_milestone(MilestoneIndex(index), None);
                        }
                        break;
                    }
                },
            }
        }

        info!("Stopped.");

        Ok(())
    }
}
