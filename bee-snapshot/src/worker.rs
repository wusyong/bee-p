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
    config::SnapshotConfig,
    constants::{ADDITIONAL_PRUNING_THRESHOLD, SOLID_ENTRY_POINT_CHECK_THRESHOLD_PAST},
    local::snapshot,
    pruning::prune_database,
};

use bee_common::{shutdown_stream::ShutdownStream, worker::Error as WorkerError};
use bee_protocol::{tangle::tangle, Milestone, MilestoneIndex};

use futures::{
    channel::mpsc,
    stream::{Fuse, StreamExt},
};
use log::{error, info};

type Receiver = ShutdownStream<Fuse<mpsc::UnboundedReceiver<SnapshotWorkerEvent>>>;

pub(crate) struct SnapshotWorkerEvent(pub(crate) Milestone);

pub(crate) struct SnapshotWorker {
    config: SnapshotConfig,
    delay: u32,
    receiver: Receiver,
}

impl SnapshotWorker {
    pub(crate) fn new(config: SnapshotConfig, receiver: Receiver) -> Self {
        let delay = std::cmp::max(
            config.pruning().delay() as u32,
            config.local().depth() as u32 + SOLID_ENTRY_POINT_CHECK_THRESHOLD_PAST + ADDITIONAL_PRUNING_THRESHOLD + 1,
        );

        Self {
            config,
            delay,
            receiver,
        }
    }

    fn should_snapshot(&self, index: MilestoneIndex) -> bool {
        let solid_index = *index;
        let snapshot_index = *tangle().get_snapshot_index();
        let pruning_index = *tangle().get_pruning_index();
        let snapshot_depth = self.config.local().depth() as u32;
        let snapshot_interval = if tangle().is_synced() {
            self.config.local().interval_synced()
        } else {
            self.config.local().interval_unsynced()
        } as u32;

        if (solid_index < snapshot_depth + snapshot_interval)
            || (solid_index - snapshot_depth) < pruning_index + 1 + SOLID_ENTRY_POINT_CHECK_THRESHOLD_PAST
        {
            // Not enough history to calculate solid entry points.
            return false;
        }

        return solid_index - (snapshot_depth + snapshot_interval) >= snapshot_index;
    }

    fn process(&mut self, milestone: Milestone) {
        if self.should_snapshot(milestone.index()) {
            if let Err(e) = snapshot(
                self.config.local().path(),
                *milestone.index() - self.config.local().depth() as u32,
            ) {
                error!("Failed to create snapshot: {:?}.", e);
            }
        }

        if self.config.pruning().enabled() && *milestone.index() > self.delay {
            if let Err(e) = prune_database(MilestoneIndex(*milestone.index() - self.delay)) {
                error!("Failed to prune database: {:?}.", e);
            }
        }
    }

    pub(crate) async fn run(mut self) -> Result<(), WorkerError> {
        info!("Running.");

        while let Some(SnapshotWorkerEvent(milestone)) = self.receiver.next().await {
            self.process(milestone);
        }

        info!("Stopped.");

        Ok(())
    }
}
