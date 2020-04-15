use crate::{
    conf::ProtocolConf,
    message::{
        Heartbeat,
        MilestoneRequest,
        TransactionBroadcast,
        TransactionRequest,
    },
    milestone::{
        MilestoneSolidifierWorker,
        MilestoneSolidifierWorkerEvent,
        MilestoneValidatorWorker,
        MilestoneValidatorWorkerEvent,
    },
    peer::Peer,
    protocol::ProtocolMetrics,
    util::WaitPriorityQueue,
    worker::{
        BroadcasterWorker,
        BroadcasterWorkerEvent,
        MilestoneRequesterWorker,
        MilestoneRequesterWorkerEntry,
        MilestoneResponderWorker,
        MilestoneResponderWorkerEvent,
        PeerWorker,
        SenderContext,
        SenderWorker,
        StatusWorker,
        TransactionRequesterWorker,
        TransactionRequesterWorkerEntry,
        TransactionResponderWorker,
        TransactionResponderWorkerEvent,
        TransactionWorker,
        TransactionWorkerEvent,
    },
};

use bee_crypto::{
    CurlP27,
    CurlP81,
    Kerl,
    SpongeType,
};
use bee_network::{
    EndpointId,
    Network,
};
use bee_signing::WotsPublicKey;

use std::{
    ptr,
    sync::{
        Arc,
        Mutex,
    },
};

use async_std::task::spawn;
use dashmap::DashMap;
use futures::{
    channel::{
        mpsc,
        oneshot,
    },
    sink::SinkExt,
};
use log::warn;

static mut PROTOCOL: *const Protocol = ptr::null();

pub struct Protocol {
    pub(crate) conf: ProtocolConf,
    pub(crate) network: Network,
    pub(crate) metrics: ProtocolMetrics,
    pub(crate) transaction_worker: (mpsc::Sender<TransactionWorkerEvent>, Mutex<Option<oneshot::Sender<()>>>),
    pub(crate) transaction_responder_worker: (
        mpsc::Sender<TransactionResponderWorkerEvent>,
        Mutex<Option<oneshot::Sender<()>>>,
    ),
    pub(crate) milestone_responder_worker: (
        mpsc::Sender<MilestoneResponderWorkerEvent>,
        Mutex<Option<oneshot::Sender<()>>>,
    ),
    pub(crate) transaction_requester_worker: (
        WaitPriorityQueue<TransactionRequesterWorkerEntry>,
        Mutex<Option<oneshot::Sender<()>>>,
    ),
    pub(crate) milestone_requester_worker: (
        WaitPriorityQueue<MilestoneRequesterWorkerEntry>,
        Mutex<Option<oneshot::Sender<()>>>,
    ),
    pub(crate) milestone_validator_worker: (
        mpsc::Sender<MilestoneValidatorWorkerEvent>,
        Mutex<Option<oneshot::Sender<()>>>,
    ),
    pub(crate) milestone_solidifier_worker: (
        mpsc::Sender<MilestoneSolidifierWorkerEvent>,
        Mutex<Option<oneshot::Sender<()>>>,
    ),
    pub(crate) broadcaster_worker: (mpsc::Sender<BroadcasterWorkerEvent>, Mutex<Option<oneshot::Sender<()>>>),
    pub(crate) status_worker: mpsc::Sender<()>,
    pub(crate) contexts: DashMap<EndpointId, SenderContext>,
}

impl Protocol {
    pub fn init(conf: ProtocolConf, network: Network) {
        if unsafe { !PROTOCOL.is_null() } {
            warn!("[Protocol ] Already initialized.");
            return;
        }

        let (transaction_worker_tx, transaction_worker_rx) = mpsc::channel(conf.workers.transaction_worker_bound);
        let (transaction_worker_shutdown_tx, transaction_worker_shutdown_rx) = oneshot::channel();

        let (transaction_responder_worker_tx, transaction_responder_worker_rx) =
            mpsc::channel(conf.workers.transaction_responder_worker_bound);
        let (transaction_responder_worker_shutdown_tx, transaction_responder_worker_shutdown_rx) = oneshot::channel();

        let (milestone_responder_worker_tx, milestone_responder_worker_rx) =
            mpsc::channel(conf.workers.milestone_responder_worker_bound);
        let (milestone_responder_worker_shutdown_tx, milestone_responder_worker_shutdown_rx) = oneshot::channel();

        let (transaction_requester_worker_shutdown_tx, transaction_requester_worker_shutdown_rx) = oneshot::channel();

        let (milestone_requester_worker_shutdown_tx, milestone_requester_worker_shutdown_rx) = oneshot::channel();

        let (milestone_validator_worker_tx, milestone_validator_worker_rx) =
            mpsc::channel(conf.workers.milestone_validator_worker_bound);
        let (milestone_validator_worker_shutdown_tx, milestone_validator_worker_shutdown_rx) = oneshot::channel();

        let (milestone_solidifier_worker_tx, milestone_solidifier_worker_rx) =
            mpsc::channel(conf.workers.milestone_solidifier_worker_bound);
        let (milestone_solidifier_worker_shutdown_tx, milestone_solidifier_worker_shutdown_rx) = oneshot::channel();

        let (broadcaster_worker_tx, broadcaster_worker_rx) = mpsc::channel(conf.workers.broadcaster_worker_bound);
        let (broadcaster_worker_shutdown_tx, broadcaster_worker_shutdown_rx) = oneshot::channel();

        let (status_worker_shutdown_tx, status_worker_shutdown_rx) = mpsc::channel(1);

        let protocol = Protocol {
            conf,
            network: network.clone(),
            metrics: ProtocolMetrics::new(),
            transaction_worker: (transaction_worker_tx, Mutex::new(Some(transaction_worker_shutdown_tx))),
            transaction_responder_worker: (
                transaction_responder_worker_tx,
                Mutex::new(Some(transaction_responder_worker_shutdown_tx)),
            ),
            milestone_responder_worker: (
                milestone_responder_worker_tx,
                Mutex::new(Some(milestone_responder_worker_shutdown_tx)),
            ),
            transaction_requester_worker: (
                WaitPriorityQueue::default(),
                Mutex::new(Some(transaction_requester_worker_shutdown_tx)),
            ),
            milestone_requester_worker: (
                WaitPriorityQueue::default(),
                Mutex::new(Some(milestone_requester_worker_shutdown_tx)),
            ),
            milestone_validator_worker: (
                milestone_validator_worker_tx,
                Mutex::new(Some(milestone_validator_worker_shutdown_tx)),
            ),
            milestone_solidifier_worker: (
                milestone_solidifier_worker_tx,
                Mutex::new(Some(milestone_solidifier_worker_shutdown_tx)),
            ),
            broadcaster_worker: (broadcaster_worker_tx, Mutex::new(Some(broadcaster_worker_shutdown_tx))),
            status_worker: status_worker_shutdown_tx,
            contexts: DashMap::new(),
        };

        unsafe {
            PROTOCOL = Box::leak(protocol.into()) as *const _;
        }

        spawn(
            TransactionWorker::new(Protocol::get().conf.workers.transaction_worker_cache)
                .run(transaction_worker_rx, transaction_worker_shutdown_rx),
        );
        spawn(TransactionResponderWorker::new().run(
            transaction_responder_worker_rx,
            transaction_responder_worker_shutdown_rx,
        ));
        spawn(
            MilestoneResponderWorker::new().run(milestone_responder_worker_rx, milestone_responder_worker_shutdown_rx),
        );
        spawn(TransactionRequesterWorker::new().run(transaction_requester_worker_shutdown_rx));
        spawn(MilestoneRequesterWorker::new().run(milestone_requester_worker_shutdown_rx));

        match Protocol::get().conf.coordinator.sponge_type {
            SpongeType::Kerl => spawn(
                MilestoneValidatorWorker::<Kerl, WotsPublicKey<Kerl>>::new()
                    .run(milestone_validator_worker_rx, milestone_validator_worker_shutdown_rx),
            ),
            SpongeType::CurlP27 => spawn(
                MilestoneValidatorWorker::<CurlP27, WotsPublicKey<CurlP27>>::new()
                    .run(milestone_validator_worker_rx, milestone_validator_worker_shutdown_rx),
            ),
            SpongeType::CurlP81 => spawn(
                MilestoneValidatorWorker::<CurlP81, WotsPublicKey<CurlP81>>::new()
                    .run(milestone_validator_worker_rx, milestone_validator_worker_shutdown_rx),
            ),
        };

        spawn(
            MilestoneSolidifierWorker::new()
                .run(milestone_solidifier_worker_rx, milestone_solidifier_worker_shutdown_rx),
        );
        spawn(BroadcasterWorker::new(network).run(broadcaster_worker_rx, broadcaster_worker_shutdown_rx));
        spawn(StatusWorker::new().run(status_worker_shutdown_rx));
    }

    pub async fn shutdown() {
        if let Ok(mut shutdown) = Protocol::get().transaction_worker.1.lock() {
            if let Some(shutdown) = shutdown.take() {
                if let Err(e) = shutdown.send(()) {
                    warn!("[Protocol ] Shutting down TransactionWorker failed: {:?}.", e);
                }
            }
        }
        if let Ok(mut shutdown) = Protocol::get().transaction_responder_worker.1.lock() {
            if let Some(shutdown) = shutdown.take() {
                if let Err(e) = shutdown.send(()) {
                    warn!("[Protocol ] Shutting down TransactionResponderWorker failed: {:?}.", e);
                }
            }
        }
        if let Ok(mut shutdown) = Protocol::get().milestone_responder_worker.1.lock() {
            if let Some(shutdown) = shutdown.take() {
                if let Err(e) = shutdown.send(()) {
                    warn!("[Protocol ] Shutting down MilestoneResponderWorker failed: {:?}.", e);
                }
            }
        }
        if let Ok(mut shutdown) = Protocol::get().transaction_requester_worker.1.lock() {
            if let Some(shutdown) = shutdown.take() {
                if let Err(e) = shutdown.send(()) {
                    warn!("[Protocol ] Shutting down TransactionRequesterWorker failed: {:?}.", e);
                }
            }
        }
        if let Ok(mut shutdown) = Protocol::get().milestone_requester_worker.1.lock() {
            if let Some(shutdown) = shutdown.take() {
                if let Err(e) = shutdown.send(()) {
                    warn!("[Protocol ] Shutting down MilestoneRequesterWorker failed: {:?}.", e);
                }
            }
        }
        if let Ok(mut shutdown) = Protocol::get().milestone_validator_worker.1.lock() {
            if let Some(shutdown) = shutdown.take() {
                if let Err(e) = shutdown.send(()) {
                    warn!("[Protocol ] Shutting down MilestoneValidatorWorker failed: {:?}.", e);
                }
            }
        }
        if let Ok(mut shutdown) = Protocol::get().milestone_solidifier_worker.1.lock() {
            if let Some(shutdown) = shutdown.take() {
                if let Err(e) = shutdown.send(()) {
                    warn!("[Protocol ] Shutting down MilestoneSolidifierWorker failed: {:?}.", e);
                }
            }
        }
        if let Ok(mut shutdown) = Protocol::get().broadcaster_worker.1.lock() {
            if let Some(shutdown) = shutdown.take() {
                if let Err(e) = shutdown.send(()) {
                    warn!("[Protocol ] Shutting down BroadcasterWorker failed: {:?}.", e);
                }
            }
        }
        if let Err(e) = Protocol::get().status_worker.clone().send(()).await {
            warn!("[Protocol ] Shutting down StatusWorker failed: {:?}.", e);
        }
    }

    pub(crate) fn get() -> &'static Protocol {
        if unsafe { PROTOCOL.is_null() } {
            panic!("Uninitialized protocol.");
        } else {
            unsafe { &*PROTOCOL }
        }
    }

    pub fn register(peer: Arc<Peer>) -> (mpsc::Sender<Vec<u8>>, oneshot::Sender<()>) {
        //TODO check if not already added ?
        // PeerWorker
        let (receiver_tx, receiver_rx) = mpsc::channel(Protocol::get().conf.workers.receiver_worker_bound);
        let (receiver_shutdown_tx, receiver_shutdown_rx) = oneshot::channel();

        spawn(PeerWorker::new(Protocol::get().network.clone(), peer).run(receiver_rx, receiver_shutdown_rx));

        (receiver_tx, receiver_shutdown_tx)
    }

    pub(crate) async fn senders_add(network: Network, peer: Arc<Peer>) {
        //TODO check if not already added

        // SenderWorker MilestoneRequest
        let (milestone_request_tx, milestone_request_rx) =
            mpsc::channel(Protocol::get().conf.workers.milestone_request_send_worker_bound);
        let (milestone_request_shutdown_tx, milestone_request_shutdown_rx) = oneshot::channel();

        spawn(
            SenderWorker::<MilestoneRequest>::new(network.clone(), peer.clone())
                .run(milestone_request_rx, milestone_request_shutdown_rx),
        );

        // SenderWorker TransactionBroadcast
        let (transaction_broadcast_tx, transaction_broadcast_rx) =
            mpsc::channel(Protocol::get().conf.workers.transaction_broadcast_send_worker_bound);
        let (transaction_broadcast_shutdown_tx, transaction_broadcast_shutdown_rx) = oneshot::channel();

        spawn(
            SenderWorker::<TransactionBroadcast>::new(network.clone(), peer.clone())
                .run(transaction_broadcast_rx, transaction_broadcast_shutdown_rx),
        );

        // SenderWorker TransactionRequest
        let (transaction_request_tx, transaction_request_rx) =
            mpsc::channel(Protocol::get().conf.workers.transaction_request_send_worker_bound);
        let (transaction_request_shutdown_tx, transaction_request_shutdown_rx) = oneshot::channel();

        spawn(
            SenderWorker::<TransactionRequest>::new(network.clone(), peer.clone())
                .run(transaction_request_rx, transaction_request_shutdown_rx),
        );

        // SenderWorker Heartbeat
        let (heartbeat_tx, heartbeat_rx) = mpsc::channel(Protocol::get().conf.workers.heartbeat_send_worker_bound);
        let (heartbeat_shutdown_tx, heartbeat_shutdown_rx) = oneshot::channel();

        spawn(SenderWorker::<Heartbeat>::new(network.clone(), peer.clone()).run(heartbeat_rx, heartbeat_shutdown_rx));

        let context = SenderContext::new(
            (milestone_request_tx, milestone_request_shutdown_tx),
            (transaction_broadcast_tx, transaction_broadcast_shutdown_tx),
            (transaction_request_tx, transaction_request_shutdown_tx),
            (heartbeat_tx, heartbeat_shutdown_tx),
        );

        Protocol::get().contexts.insert(peer.epid, context);
    }

    pub(crate) async fn senders_remove(epid: &EndpointId) {
        if let Some((_, context)) = Protocol::get().contexts.remove(epid) {
            if let Err(_) = context.milestone_request.1.send(()) {
                warn!("[Protocol ] Shutting down MilestoneRequest SenderWorker failed.");
            }
            if let Err(_) = context.transaction_broadcast.1.send(()) {
                warn!("[Protocol ] Shutting down TransactionBroadcast SenderWorker failed.");
            }
            if let Err(_) = context.transaction_request.1.send(()) {
                warn!("[Protocol ] Shutting down TransactionRequest SenderWorker failed.");
            }
            if let Err(_) = context.heartbeat.1.send(()) {
                warn!("[Protocol ] Shutting down Heartbeat SenderWorker failed.");
            }
        }
    }
}