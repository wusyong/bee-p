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

#![allow(clippy::assertions_on_constants)]

use crate::{
    message::{uncompress_transaction_bytes, Transaction as TransactionMessage},
    protocol::Protocol,
    worker::transaction::{HashCache, ProcessorWorker, ProcessorWorkerEvent},
};

use bee_common::{shutdown_stream::ShutdownStream, worker::Error as WorkerError};
use bee_common_ext::{node::Node, worker::Worker};
use bee_crypto::ternary::{
    sponge::{BatchHasher, CurlPRounds, BATCH_SIZE},
    Hash,
};
use bee_network::EndpointId;
use bee_ternary::{T5B1Buf, TritBuf, Trits, T5B1};
use bee_transaction::bundled::{BundledTransactionField, TRANSACTION_TRIT_LEN};

use async_trait::async_trait;
use bytemuck::cast_slice;
use futures::{
    stream::{Fuse, Stream, StreamExt},
    task::{Context, Poll},
};
use log::{info, trace, warn};
use pin_project::pin_project;

use std::{any::TypeId, pin::Pin};

// If a batch has less than this number of transactions, the regular CurlP hasher is used instead
// of the batched one.
const BATCH_SIZE_THRESHOLD: usize = 3;

pub(crate) struct HasherWorkerEvent {
    pub(crate) from: EndpointId,
    pub(crate) transaction_message: TransactionMessage,
}

pub(crate) struct HasherWorker {
    pub(crate) tx: flume::Sender<HasherWorkerEvent>,
}

fn trigger_hashing(
    batch_size: usize,
    receiver: &mut BatchStream,
    processor_worker: &mut flume::Sender<ProcessorWorkerEvent>,
) {
    if batch_size < BATCH_SIZE_THRESHOLD {
        let hashes = receiver.hasher.hash_unbatched();
        send_hashes(hashes, &mut receiver.events, processor_worker);
    } else {
        let hashes = receiver.hasher.hash_batched();
        send_hashes(hashes, &mut receiver.events, processor_worker);
    }
    // FIXME: we could store the fraction of times we use the batched hasher
}

fn send_hashes(
    hashes: impl Iterator<Item = TritBuf>,
    events: &mut Vec<HasherWorkerEvent>,
    processor_worker: &mut flume::Sender<ProcessorWorkerEvent>,
) {
    for (
        HasherWorkerEvent {
            from,
            transaction_message,
        },
        hash,
    ) in events.drain(..).zip(hashes)
    {
        if let Err(e) = processor_worker.send(ProcessorWorkerEvent {
            hash: Hash::from_inner_unchecked(hash),
            from,
            transaction_message,
        }) {
            warn!("Sending event to the processor worker failed: {}.", e);
        }
    }
}

#[async_trait]
impl<N: Node> Worker<N> for HasherWorker {
    type Config = usize;
    type Error = WorkerError;

    fn dependencies() -> &'static [TypeId] {
        Box::leak(Box::from(vec![TypeId::of::<ProcessorWorker>()]))
    }

    async fn start(node: &mut N, config: Self::Config) -> Result<Self, Self::Error> {
        let (tx, rx) = flume::unbounded();
        let mut processor_worker = node.worker::<ProcessorWorker>().unwrap().tx.clone();

        node.spawn::<Self, _, _>(|shutdown| async move {
            let mut receiver = BatchStream::new(config, ShutdownStream::new(shutdown, rx.into_stream()));

            info!("Running.");

            while let Some(batch_size) = receiver.next().await {
                trigger_hashing(batch_size, &mut receiver, &mut processor_worker);
            }

            info!("Stopped.");
        });

        Ok(Self { tx })
    }
}

#[pin_project(project = BatchStreamProj)]
pub(crate) struct BatchStream {
    #[pin]
    receiver: ShutdownStream<Fuse<flume::r#async::RecvStream<'static, HasherWorkerEvent>>>,
    cache: HashCache,
    hasher: BatchHasher<T5B1Buf>,
    events: Vec<HasherWorkerEvent>,
}

impl BatchStream {
    pub(crate) fn new(
        cache_size: usize,
        receiver: ShutdownStream<Fuse<flume::r#async::RecvStream<'static, HasherWorkerEvent>>>,
    ) -> Self {
        assert!(BATCH_SIZE_THRESHOLD <= BATCH_SIZE);
        Self {
            receiver,
            cache: HashCache::new(cache_size),
            hasher: BatchHasher::new(TRANSACTION_TRIT_LEN, CurlPRounds::Rounds81),
            events: Vec::with_capacity(BATCH_SIZE),
        }
    }
}

impl Stream for BatchStream {
    type Item = usize;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        // We need to do this because `receiver` needs to be pinned to be polled.
        let BatchStreamProj {
            mut receiver,
            hasher,
            events,
            cache,
            ..
        } = self.project();

        // We loop until we have `BATCH_SIZE` transactions or `stream.poll_next(cx)` returns
        // pending.
        loop {
            let batch_size = hasher.len();
            // If we already have `BATCH_SIZE` transactions, we are ready.
            if batch_size == BATCH_SIZE {
                return Poll::Ready(Some(BATCH_SIZE));
            }
            // Otherwise we need to check if there are transactions inside the `receiver` stream
            // that we could include in the current batch.
            match receiver.as_mut().poll_next(cx) {
                Poll::Pending => {
                    return if batch_size == 0 {
                        // If the stream is not ready yet and the current batch is empty we have to
                        // wait. Otherwise, we would end up hashing an empty batch.
                        Poll::Pending
                    } else {
                        // If the stream is not ready yet, but we have transactions in the batch,
                        // we can process them instead of waiting.
                        Poll::Ready(Some(batch_size))
                    };
                }
                Poll::Ready(Some(event)) => {
                    // If the transaction was already received, we skip it and poll again.
                    if !cache.insert(&event.transaction_message.bytes) {
                        trace!("Transaction already received.");
                        Protocol::get().metrics.known_transactions_inc();
                        continue;
                    }
                    // Given that the current batch has less than `BATCH_SIZE` transactions. We can
                    // add the transaction in the current event to the batch.
                    let transaction_bytes = uncompress_transaction_bytes(&event.transaction_message.bytes);

                    let trits = Trits::<T5B1>::try_from_raw(cast_slice(&transaction_bytes), TRANSACTION_TRIT_LEN)
                        .unwrap()
                        .to_buf::<T5B1Buf>();

                    hasher.add(trits);
                    events.push(event);
                    // If after adding the transaction to the batch its size is `BATCH_SIZE` we are
                    // ready to hash.
                    if batch_size == BATCH_SIZE - 1 {
                        return Poll::Ready(Some(BATCH_SIZE));
                    }
                }
                Poll::Ready(None) => {
                    // If the `receiver` stream ended, it means that either we should shutdown or
                    // the other side of the channel disconnected. In either case, we end this
                    // stream too without hashing the pending batch we have.
                    return Poll::Ready(None);
                }
            }
        }
    }
}
