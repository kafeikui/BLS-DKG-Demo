use super::cache::{BlockInfoFetcher, BlockInfoUpdater, GroupInfoFetcher, GroupInfoUpdater};
use super::client::{
    ControllerMockHelper, ControllerTransactions, ControllerViews, MockControllerClient,
};
use super::errors::{NodeError, NodeResult};
use super::types::Group;
use super::{
    cache::{
        InMemoryBlockInfoCache, InMemoryGroupInfoCache, InMemoryNodeInfoCache, NodeInfoFetcher,
    },
    dkg::{DKGCore, MockDKGCore},
    types::DKGTask,
};
use async_trait::async_trait;
use futures::TryFutureExt;
use parking_lot::RwLock;
use rand::RngCore;
use std::sync::Arc;

pub const DEFAULT_DKG_TIMEOUT_DURATION: usize = 30 * 3;

#[async_trait]
pub trait StartingGroupingListener<F, R> {
    async fn start(&self) -> NodeResult<()>;

    async fn handle(
        task: DKGTask,
        rng: F,
        node_cache: Arc<RwLock<impl NodeInfoFetcher + Send + Sync + 'async_trait>>,
        group_cache_fetcher: Arc<RwLock<impl GroupInfoFetcher + Send + Sync + 'async_trait>>,
        group_cache_updater: Arc<RwLock<impl GroupInfoUpdater + Send + Sync + 'async_trait>>,
    ) -> NodeResult<usize>
    where
        R: RngCore,
        F: Fn() -> R + 'static;
}

pub struct MockStartingGroupingListener<F: Fn() -> R, R: RngCore> {
    rng: F,
    block_cache: Arc<RwLock<InMemoryBlockInfoCache>>,
    node_cache: Arc<RwLock<InMemoryNodeInfoCache>>,
    group_cache: Arc<RwLock<InMemoryGroupInfoCache>>,
}

impl<F: Fn() -> R, R: RngCore> MockStartingGroupingListener<F, R> {
    pub fn new(
        rng: F,
        block_cache: Arc<RwLock<InMemoryBlockInfoCache>>,
        node_cache: Arc<RwLock<InMemoryNodeInfoCache>>,
        group_cache: Arc<RwLock<InMemoryGroupInfoCache>>,
    ) -> Self {
        MockStartingGroupingListener {
            rng,
            block_cache,
            node_cache,
            group_cache,
        }
    }
}

#[async_trait]
impl<F: Fn() -> R + Send + Sync + Copy + 'static, R: RngCore + 'static>
    StartingGroupingListener<F, R> for MockStartingGroupingListener<F, R>
{
    async fn start(&self) -> NodeResult<()> {
        let controller_address = String::from("http://[::1]:50052");

        let id_address = self.node_cache.read().get_id_address().to_string();

        let mut client = MockControllerClient::new(controller_address, id_address).await?;

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

                        let block_cache = self.block_cache.clone();

                        let node_cache = self.node_cache.clone();

                        let group_cache = self.group_cache.clone();

                        let rng = self.rng;

                        tokio::spawn(async move {
                            if let Err(e) = MockStartingGroupingListener::handle(
                                dkg_task,
                                rng,
                                node_cache.clone(),
                                group_cache.clone(),
                                group_cache.clone(),
                            )
                            .and_then(move |timeout_block_height| {
                                let end_grouping_listener = MockEndGroupingListener::new(
                                    block_cache.clone(),
                                    node_cache,
                                    group_cache,
                                );
                                end_grouping_listener.start(timeout_block_height)
                            })
                            .await
                            {
                                println!("{:?}", e);
                            }
                        });
                    }
                }
            }
            print!(".");

            tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
        }
    }

    async fn handle(
        task: DKGTask,
        rng: F,
        node_cache: Arc<RwLock<impl NodeInfoFetcher + Send + Sync + 'async_trait>>,
        group_cache_fetcher: Arc<RwLock<impl GroupInfoFetcher + Send + Sync + 'async_trait>>,
        group_cache_updater: Arc<RwLock<impl GroupInfoUpdater + Send + Sync + 'async_trait>>,
    ) -> NodeResult<usize>
    where
        R: RngCore,
        F: Fn() -> R + Send + 'async_trait,
    {
        let controller_address = String::from("http://[::1]:50052");

        let id_address = node_cache.read().get_id_address().to_string();

        let mut controller_client =
            MockControllerClient::new(controller_address, id_address).await?;

        let mut dkg_core = MockDKGCore {};

        let dkg_private_key = *node_cache.read().get_dkg_private_key()?;

        let id_address = node_cache.read().get_id_address().to_string();

        let task_group_index = task.group_index;

        let task_epoch = task.epoch;

        let timeout_block_height = task.assignment_block_height + DEFAULT_DKG_TIMEOUT_DURATION;

        //TODO retry if error happens
        let output = dkg_core
            .run_dkg(dkg_private_key, id_address, task, rng, group_cache_fetcher)
            .await?;

        let (public_key, partial_public_key, disqualified_nodes) = group_cache_updater
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
    block_cache: Arc<RwLock<InMemoryBlockInfoCache>>,
    node_cache: Arc<RwLock<InMemoryNodeInfoCache>>,
    group_cache: Arc<RwLock<InMemoryGroupInfoCache>>,
}

impl MockEndGroupingListener {
    pub fn new(
        block_cache: Arc<RwLock<InMemoryBlockInfoCache>>,
        node_cache: Arc<RwLock<InMemoryNodeInfoCache>>,
        group_cache: Arc<RwLock<InMemoryGroupInfoCache>>,
    ) -> Self {
        MockEndGroupingListener {
            block_cache,
            node_cache,
            group_cache,
        }
    }
}

#[async_trait]
impl EndGroupingListener for MockEndGroupingListener {
    async fn start(self, timeout_block_height: usize) -> NodeResult<()> {
        let controller_address = String::from("http://[::1]:50052");

        let id_address = self.node_cache.read().get_id_address().to_string();

        let mut client = MockControllerClient::new(controller_address, id_address).await?;

        let group_index = self.group_cache.read().get_index()?;

        loop {
            let group = client.get_group(group_index).await?;

            match self.handle(group) {
                Ok(()) => {
                    println!("DKG task execute successfully! Ready to handle bls task.");

                    return Ok(());
                }

                Err(NodeError::GroupWaitingForConsensus) => {
                    let block_height = self.block_cache.read().get_block_height();

                    if block_height > timeout_block_height {
                        client.check_dkg_state(group_index).await?;

                        println!(
                            "DKG task timeout in committing output phase. Wait for next task..."
                        );

                        return Ok(());
                    }
                }

                Err(e) => return Err(e),
            }

            tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
        }
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
    async fn start(&self) -> NodeResult<()>;

    fn handle(&self, block_height: usize) -> NodeResult<()>;
}

pub struct MockBlockListener {
    node_cache: Arc<RwLock<InMemoryBlockInfoCache>>,
}

impl MockBlockListener {
    pub fn new(node_cache: Arc<RwLock<InMemoryBlockInfoCache>>) -> Self {
        MockBlockListener { node_cache }
    }
}

#[async_trait]
impl BlockListener for MockBlockListener {
    async fn start(&self) -> NodeResult<()> {
        let controller_address = String::from("http://[::1]:50052");

        let mut client = MockControllerClient::new(controller_address, "".to_string()).await?;

        loop {
            let block_height = client.mine(1).await?;

            self.handle(block_height)?;

            tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
        }
    }

    fn handle(&self, block_height: usize) -> NodeResult<()> {
        self.node_cache.write().set_block_height(block_height);

        Ok(())
    }
}
