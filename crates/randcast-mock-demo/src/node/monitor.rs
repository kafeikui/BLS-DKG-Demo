use super::cache::{
    BlockInfoFetcher, BlockInfoUpdater, GroupInfoFetcher, GroupInfoUpdater, InMemoryBLSTasksQueue,
    InMemorySignatureResultCache, SignatureResultCache, SignatureResultCacheUpdater,
};
use super::controller_client::{
    ControllerMockHelper, ControllerTransactions, ControllerViews, MockControllerClient,
    MockCoordinatorClient,
};
use super::errors::{NodeError, NodeResult};
use super::types::{Group, SignatureTask, TaskType};
use super::{
    bls::{BLSCore, MockBLSCore},
    cache::{
        InMemoryBlockInfoCache, InMemoryGroupInfoCache, InMemoryNodeInfoCache, NodeInfoFetcher,
    },
    dkg::{DKGCore, MockDKGCore},
    types::DKGTask,
};
use crate::node::cache::{BLSTasksFetcher, BLSTasksUpdater, SignatureResultCacheFetcher};
use crate::node::committer_client::{CommitterService, MockCommitterClient};
use crate::node::committer_server;
use async_trait::async_trait;
use parking_lot::RwLock;
use rand::RngCore;
use std::io::{self, Write};
use std::sync::Arc;
use tokio::task::JoinHandle;

pub const DEFAULT_DKG_TIMEOUT_DURATION: usize = 10 * 4;

#[async_trait]
pub trait StartingGroupingListener<F, R> {
    async fn start(self) -> NodeResult<()>;

    async fn handle(&self, task: DKGTask) -> NodeResult<usize>
    where
        R: RngCore,
        F: Fn() -> R + 'static;
}

pub struct MockStartingGroupingListener<F: Fn() -> R, R: RngCore> {
    rng: F,
    block_cache: Arc<RwLock<InMemoryBlockInfoCache>>,
    node_cache: Arc<RwLock<InMemoryNodeInfoCache>>,
    group_cache: Arc<RwLock<InMemoryGroupInfoCache>>,
    bls_tasks_cache: Arc<RwLock<InMemoryBLSTasksQueue<SignatureTask>>>,
    committer_cache: Arc<RwLock<InMemorySignatureResultCache>>,
}

impl<F: Fn() -> R, R: RngCore> MockStartingGroupingListener<F, R> {
    pub fn new(
        rng: F,
        block_cache: Arc<RwLock<InMemoryBlockInfoCache>>,
        node_cache: Arc<RwLock<InMemoryNodeInfoCache>>,
        group_cache: Arc<RwLock<InMemoryGroupInfoCache>>,
        bls_tasks_cache: Arc<RwLock<InMemoryBLSTasksQueue<SignatureTask>>>,
        committer_cache: Arc<RwLock<InMemorySignatureResultCache>>,
    ) -> Self {
        MockStartingGroupingListener {
            rng,
            block_cache,
            node_cache,
            group_cache,
            bls_tasks_cache,
            committer_cache,
        }
    }
}

#[async_trait]
impl<F: Fn() -> R + Send + Sync + Copy + 'static, R: RngCore + 'static>
    StartingGroupingListener<F, R> for MockStartingGroupingListener<F, R>
{
    async fn start(self) -> NodeResult<()> {
        let id_address = self.node_cache.read().get_id_address().to_string();

        let controller_address = self
            .node_cache
            .read()
            .get_controller_rpc_endpoint()
            .to_string();

        let mut client =
            MockControllerClient::new(controller_address.clone(), id_address.clone()).await?;

        loop {
            if let Ok(dkg_task) = client.emit_dkg_task().await {
                if let Some((_, node_index)) = dkg_task
                    .members
                    .iter()
                    .find(|(id_address, _)| *id_address == self.node_cache.read().get_id_address())
                {
                    let cache_index = self.group_cache.read().get_index().unwrap_or(0);

                    let cache_epoch = self.group_cache.read().get_epoch().unwrap_or(0);

                    let task_group_index = dkg_task.group_index;

                    let task_epoch = dkg_task.epoch;

                    if cache_index != task_group_index || cache_epoch != task_epoch {
                        self.group_cache
                            .write()
                            .save_task_info(*node_index, dkg_task.clone())?;

                        println!(
                            "received new dkg_task: index:{} epoch:{}, start handling...",
                            task_group_index, task_epoch
                        );

                        let id_address = id_address.clone();

                        let controller_address = controller_address.clone();

                        let node_rpc_endpoint =
                            self.node_cache.read().get_node_rpc_endpoint().to_string();

                        let block_cache = self.block_cache.clone();

                        let group_cache = self.group_cache.clone();

                        let bls_tasks_cache = self.bls_tasks_cache.clone();

                        let committer_cache = self.committer_cache.clone();

                        match self.handle(dkg_task).await {
                            Ok(timeout_block_height) => {
                                tokio::spawn(async move {
                                    let end_grouping_listener = MockEndGroupingListener::new(
                                        id_address,
                                        controller_address,
                                        node_rpc_endpoint,
                                        block_cache,
                                        group_cache,
                                        bls_tasks_cache,
                                        committer_cache,
                                    );
                                    if let Err(e) =
                                        end_grouping_listener.start(timeout_block_height).await
                                    {
                                        println!("{:?}", e);
                                    }
                                });
                            }
                            Err(e) => {
                                println!("{:?}", e);
                            }
                        }
                    }
                }
            }
            print!(".");
            io::stdout().flush().unwrap();

            tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
        }
    }

    async fn handle(&self, task: DKGTask) -> NodeResult<usize>
    where
        R: RngCore,
        F: Fn() -> R + Send + 'async_trait,
    {
        let controller_address = self
            .node_cache
            .read()
            .get_controller_rpc_endpoint()
            .to_string();

        let coordinator_rpc_endpoint = self
            .node_cache
            .read()
            .get_controller_rpc_endpoint()
            .to_string();

        let id_address = self.node_cache.read().get_id_address().to_string();

        let node_rpc_endpoint = self.node_cache.read().get_node_rpc_endpoint().to_string();

        let mut controller_client =
            MockControllerClient::new(controller_address, id_address).await?;

        let mut dkg_core = MockDKGCore {};

        let dkg_private_key = *self.node_cache.read().get_dkg_private_key()?;

        let id_address = self.node_cache.read().get_id_address().to_string();

        let task_group_index = task.group_index;

        let task_epoch = task.epoch;

        let timeout_block_height = task.assignment_block_height + DEFAULT_DKG_TIMEOUT_DURATION;

        let group_cache_fetcher = self.group_cache.clone();

        //TODO retry if error happens
        let coordinator_client = MockCoordinatorClient::new(
            coordinator_rpc_endpoint,
            id_address,
            task.group_index,
            task.epoch,
        )
        .await?;

        let output = dkg_core
            .run_dkg(
                dkg_private_key,
                node_rpc_endpoint,
                task,
                self.rng,
                coordinator_client,
                group_cache_fetcher,
            )
            .await?;

        let (public_key, partial_public_key, disqualified_nodes) = self
            .group_cache
            .write()
            .save_output(task_group_index, task_epoch, output)?;

        controller_client
            .commit_dkg(
                task_group_index,
                task_epoch,
                bincode::serialize(&public_key).unwrap(),
                bincode::serialize(&partial_public_key).unwrap(),
                disqualified_nodes,
            )
            .await?;

        Ok(timeout_block_height)
    }
}

#[async_trait]
trait EndGroupingListener {
    async fn start(self, timeout_block_height: usize) -> NodeResult<()>;

    fn handle(&self, group: Group) -> NodeResult<()>;
}

pub struct MockEndGroupingListener {
    id_address: String,
    controller_address: String,
    node_rpc_endpoint: String,
    block_cache: Arc<RwLock<InMemoryBlockInfoCache>>,
    group_cache: Arc<RwLock<InMemoryGroupInfoCache>>,
    bls_tasks_cache: Arc<RwLock<InMemoryBLSTasksQueue<SignatureTask>>>,
    committer_cache: Arc<RwLock<InMemorySignatureResultCache>>,
}

impl MockEndGroupingListener {
    pub fn new(
        id_address: String,
        controller_address: String,
        node_rpc_endpoint: String,
        block_cache: Arc<RwLock<InMemoryBlockInfoCache>>,
        group_cache: Arc<RwLock<InMemoryGroupInfoCache>>,
        bls_tasks_cache: Arc<RwLock<InMemoryBLSTasksQueue<SignatureTask>>>,
        committer_cache: Arc<RwLock<InMemorySignatureResultCache>>,
    ) -> Self {
        MockEndGroupingListener {
            id_address,
            controller_address,
            node_rpc_endpoint,
            block_cache,
            group_cache,
            bls_tasks_cache,
            committer_cache,
        }
    }
}

#[async_trait]
impl EndGroupingListener for MockEndGroupingListener {
    async fn start(self, timeout_block_height: usize) -> NodeResult<()> {
        let mut client =
            MockControllerClient::new(self.controller_address.clone(), self.id_address.clone())
                .await?;

        let group_index = self.group_cache.read().get_index()?;

        let is_post_dkg_handle_success = false;

        let mut block_height = self.block_cache.read().get_block_height();

        while block_height <= timeout_block_height {
            let group = client.get_group(group_index).await?;

            if let Ok(()) = self.handle(group) {
                println!("DKG task execute successfully!");

                let mut listener_tasks: Vec<JoinHandle<()>> = Vec::new();

                if self.group_cache.read().is_committer(&self.id_address)? {
                    let id_address = self.id_address.clone();

                    let controller_address = self.controller_address.clone();

                    let committer_cache = self.committer_cache.clone();

                    let signature_aggregation_listener_task = tokio::spawn(async move {
                        let signature_aggregation_listener = MockSignatureAggregationListener::new(
                            id_address,
                            controller_address,
                            committer_cache,
                        );
                        if let Err(e) = signature_aggregation_listener.start().await {
                            println!("{:?}", e);
                        }
                    });

                    listener_tasks.push(signature_aggregation_listener_task);

                    let group_cache = self.group_cache.clone();
                    let endpoint = self.node_rpc_endpoint.clone();
                    let group_cache_for_committer_server = self.group_cache.clone();
                    let committer_cache_for_committer_server = self.committer_cache.clone();
                    tokio::spawn(async move {
                        if let Err(e) = committer_server::start_committer_server(
                            endpoint,
                            group_cache_for_committer_server,
                            committer_cache_for_committer_server,
                            async {
                                loop {
                                    match group_cache.clone().read().get_state() {
                                        Err(_) => {
                                            break;
                                        }
                                        Ok(false) => {
                                            break;
                                        }
                                        _ => {}
                                    }
                                    tokio::time::sleep(std::time::Duration::from_millis(2000))
                                        .await;
                                }
                            },
                        )
                        .await
                        {
                            println!("{:?}", e);
                        };
                    });
                }

                let id_address = self.id_address.clone();

                let controller_address = self.controller_address.clone();

                let block_cache = self.block_cache.clone();

                let group_cache = self.group_cache.clone();

                let bls_tasks_cache = self.bls_tasks_cache.clone();

                let committer_cache = self.committer_cache.clone();

                let bls_task_listener_task = tokio::spawn(async move {
                    let mut bls_task_listener = MockBLSTaskListener::new(
                        id_address,
                        controller_address,
                        block_cache,
                        group_cache,
                        bls_tasks_cache,
                        committer_cache,
                    );
                    if let Err(e) = bls_task_listener.init().await {
                        println!("{:?}", e);
                    }
                    if let Err(e) = bls_task_listener.start().await {
                        println!("{:?}", e);
                    }
                });

                listener_tasks.push(bls_task_listener_task);

                let group_cache = self.group_cache.clone();
                tokio::spawn(async move {
                    loop {
                        match group_cache.clone().read().get_state() {
                            Err(_) => {
                                for task in listener_tasks {
                                    task.abort();
                                }
                                break;
                            }
                            Ok(false) => {
                                for task in listener_tasks {
                                    task.abort();
                                }
                                break;
                            }
                            _ => {}
                        }
                    }
                });
            }

            block_height = self.block_cache.read().get_block_height();

            tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
        }

        client.check_dkg_state(group_index).await?;

        if !is_post_dkg_handle_success {
            println!("DKG task timeout in committing output phase. Wait for next task...");
        }

        return Ok(());
    }

    fn handle(&self, group: Group) -> NodeResult<()> {
        if group.state {
            self.group_cache
                .write()
                .save_committers(group.index, group.epoch, group.committers)?;

            return Ok(());
        }

        Err(NodeError::GroupWaitingForConsensus)
    }
}

#[async_trait]
pub trait BlockListener {
    async fn start(self) -> NodeResult<()>;

    fn handle(&self, block_height: usize) -> NodeResult<()>;
}

pub struct MockBlockListener {
    controller_address: String,
    block_cache: Arc<RwLock<InMemoryBlockInfoCache>>,
}

impl MockBlockListener {
    pub fn new(
        controller_address: String,
        node_cache: Arc<RwLock<InMemoryBlockInfoCache>>,
    ) -> Self {
        MockBlockListener {
            controller_address,
            block_cache: node_cache,
        }
    }
}

#[async_trait]
impl BlockListener for MockBlockListener {
    async fn start(self) -> NodeResult<()> {
        let mut client =
            MockControllerClient::new(self.controller_address.clone(), "".to_string()).await?;

        loop {
            let block_height = client.mine(1).await?;

            self.handle(block_height)?;

            tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
        }
    }

    fn handle(&self, block_height: usize) -> NodeResult<()> {
        self.block_cache.write().set_block_height(block_height);

        Ok(())
    }
}

#[async_trait]
pub trait BLSTaskListener {
    async fn init(&mut self) -> NodeResult<()>;

    async fn start(mut self) -> NodeResult<()>;

    fn handle(
        &self,
        task: &SignatureTask,
        group_cache_fetcher: Arc<RwLock<impl GroupInfoFetcher + Send + Sync>>,
    ) -> NodeResult<Vec<u8>>;
}

pub struct MockBLSTaskListener {
    id_address: String,
    adapter_address: String,
    block_cache: Arc<RwLock<InMemoryBlockInfoCache>>,
    group_cache: Arc<RwLock<InMemoryGroupInfoCache>>,
    bls_tasks_cache: Arc<RwLock<InMemoryBLSTasksQueue<SignatureTask>>>,
    committer_cache: Arc<RwLock<InMemorySignatureResultCache>>,
    committer_clients: Vec<MockCommitterClient>,
}

impl MockBLSTaskListener {
    pub fn new(
        id_address: String,
        adapter_address: String,
        block_cache: Arc<RwLock<InMemoryBlockInfoCache>>,
        group_cache: Arc<RwLock<InMemoryGroupInfoCache>>,
        bls_tasks_cache: Arc<RwLock<InMemoryBLSTasksQueue<SignatureTask>>>,
        committer_cache: Arc<RwLock<InMemorySignatureResultCache>>,
    ) -> Self {
        MockBLSTaskListener {
            id_address,
            adapter_address,
            block_cache,
            group_cache,
            bls_tasks_cache,
            committer_cache,
            committer_clients: Vec::new(),
        }
    }
}

#[async_trait]
impl BLSTaskListener for MockBLSTaskListener {
    async fn init(&mut self) -> NodeResult<()> {
        let state = self.group_cache.read().get_state()?;

        if !state {
            return Err(NodeError::GroupNotReady);
        }

        println!("ready to handle bls task.");

        let mut committers = self
            .group_cache
            .read()
            .get_committers()?
            .iter()
            .map(|c| c.to_string())
            .collect::<Vec<_>>();

        committers.retain(|c| *c != self.id_address);

        for committer in committers {
            let endpoint = self
                .group_cache
                .read()
                .get_member(&committer)?
                .rpc_endpint
                .as_ref()
                .unwrap()
                .to_string();

            // we retry some times here as building tonic connection needs the target rpc server available
            let mut i = 0;
            while i < 3 {
                if let Ok(committer_client) =
                    MockCommitterClient::new(self.id_address.clone(), endpoint.clone()).await
                {
                    self.committer_clients.push(committer_client);
                    break;
                }
                i += 1;
                tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
            }
        }

        Ok(())
    }

    async fn start(mut self) -> NodeResult<()> {
        let mut client =
            MockControllerClient::new(self.adapter_address.clone(), self.id_address.clone())
                .await?;

        loop {
            let task_reply = client.emit_signature_task().await;

            if let Err(NodeError::NoTaskAvailable) = task_reply {
                tokio::time::sleep(std::time::Duration::from_millis(2000)).await;
                continue;
            }

            let task = task_reply.unwrap();

            let SignatureTask {
                index: task_index,
                message: task_message,
                group_index: _,
                assignment_block_height: _,
            } = task.clone();

            if self.bls_tasks_cache.read().contains(task_index) {
                tokio::time::sleep(std::time::Duration::from_millis(2000)).await;
                continue;
            }

            println!(
                "received new signature task. index: {}, message: {}",
                task_index, task_message
            );

            self.bls_tasks_cache.write().add(task.clone())?;

            let current_group_index = self.group_cache.read().get_index()?;

            let current_block_height = self.block_cache.read().get_block_height();

            let available_tasks = self
                .bls_tasks_cache
                .write()
                .check_and_get_available_tasks(current_block_height, current_group_index);

            let group_cache = self.group_cache.clone();

            for task in available_tasks {
                match self.handle(&task, group_cache.clone()) {
                    Ok(partial_signature) => {
                        let threshold = self.group_cache.read().get_threshold()?;

                        if self.group_cache.read().is_committer(&self.id_address)? {
                            if !self.committer_cache.read().contains(task_index) {
                                self.committer_cache.write().add(
                                    current_group_index,
                                    task_index,
                                    threshold,
                                )?;
                            }

                            self.committer_cache.write().add_partial_signature(
                                task_index,
                                self.id_address.clone(),
                                partial_signature.clone(),
                            )?;
                        }

                        for committer in self.committer_clients.iter_mut() {
                            committer
                                .commit_partial_signature(
                                    TaskType::Randomness,
                                    task.message.as_bytes().to_vec(),
                                    task_index,
                                    partial_signature.clone(),
                                )
                                .await?;
                        }
                    }

                    Err(e) => {
                        println!("{:?}", e);
                    }
                }
            }

            tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
        }
    }

    fn handle(
        &self,
        task: &SignatureTask,
        group_cache_fetcher: Arc<RwLock<impl GroupInfoFetcher + Send + Sync>>,
    ) -> NodeResult<Vec<u8>> {
        let fetcher = group_cache_fetcher.read();

        let share = fetcher.get_secret_share()?;

        let bls_core = MockBLSCore {};

        let partial_signature = bls_core.partial_sign(share, task.message.as_bytes())?;

        Ok(partial_signature)
    }
}

#[async_trait]
pub trait SignatureAggregationListener {
    async fn start(self) -> NodeResult<()>;
}

pub struct MockSignatureAggregationListener {
    id_address: String,
    controller_address: String,
    committer_cache: Arc<RwLock<InMemorySignatureResultCache>>,
}

impl MockSignatureAggregationListener {
    pub fn new(
        id_address: String,
        controller_address: String,
        committer_cache: Arc<RwLock<InMemorySignatureResultCache>>,
    ) -> Self {
        MockSignatureAggregationListener {
            id_address,
            controller_address,
            committer_cache,
        }
    }
}

#[async_trait]
impl SignatureAggregationListener for MockSignatureAggregationListener {
    async fn start(self) -> NodeResult<()> {
        let mut client =
            MockControllerClient::new(self.controller_address, self.id_address).await?;

        loop {
            let ready_signatures = self
                .committer_cache
                .write()
                .get_ready_to_commit_signatures();

            for signature in ready_signatures {
                let SignatureResultCache {
                    group_index,
                    signature_index,
                    threshold,
                    partial_signatures,
                } = signature;

                let bls_core = MockBLSCore {};

                let signature = bls_core.aggregate(
                    threshold,
                    &partial_signatures.values().cloned().collect::<Vec<_>>(),
                )?;

                if !client
                    .get_signature_task_completion_state(signature_index)
                    .await?
                {
                    match client
                        .fulfill_randomness(
                            group_index,
                            signature_index,
                            signature.clone(),
                            partial_signatures,
                        )
                        .await
                    {
                        Ok(()) => {
                            println!("fulfill randomness successfully! signature index: {}, group_index: {}, signature: {}",
                            signature_index, group_index, hex::encode(signature));
                        }
                        Err(e) => {
                            println!("{:?}", e);
                        }
                    }
                }

                self.committer_cache.write().remove(signature_index)?;
            }

            tokio::time::sleep(std::time::Duration::from_millis(2000)).await;
        }
    }
}
