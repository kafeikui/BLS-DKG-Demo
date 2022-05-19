use std::collections::HashMap;

use super::{
    errors::{NodeError, NodeResult},
    types::{DKGTask, Group, Member, SignatureTask},
};
use dkg_core::primitives::DKGOutput;
use threshold_bls::group::Element;
use threshold_bls::{
    curve::bls12381::{Curve, Scalar, G1},
    sig::Share,
};

pub trait BlockInfoFetcher {
    fn get_block_height(&self) -> usize;
}

pub trait BlockInfoUpdater {
    fn set_block_height(&mut self, block_height: usize);
}

#[derive(Default)]
pub struct InMemoryBlockInfoCache {
    block_height: usize,
}

impl InMemoryBlockInfoCache {
    pub fn new() -> Self {
        InMemoryBlockInfoCache { block_height: 0 }
    }
}

impl BlockInfoFetcher for InMemoryBlockInfoCache {
    fn get_block_height(&self) -> usize {
        self.block_height
    }
}

impl BlockInfoUpdater for InMemoryBlockInfoCache {
    fn set_block_height(&mut self, block_height: usize) {
        self.block_height = block_height;
    }
}

pub trait NodeInfoFetcher {
    fn get_private_key(&self) -> &[u8];

    fn get_id_address(&self) -> &str;

    fn get_node_rpc_endpoint(&self) -> &str;

    fn get_controller_rpc_endpoint(&self) -> &str;

    fn get_dkg_private_key(&self) -> NodeResult<&Scalar>;

    fn get_dkg_public_key(&self) -> NodeResult<&G1>;
}

pub struct InMemoryNodeInfoCache {
    private_key: Vec<u8>,
    id_address: String,
    node_rpc_endpoint: String,
    controller_rpc_endpoint: String,
    dkg_private_key: Option<Scalar>,
    dkg_public_key: Option<G1>,
}

impl InMemoryNodeInfoCache {
    pub fn new(
        id_address: String,
        node_rpc_endpoint: String,
        controller_rpc_endpoint: String,
        dkg_private_key: Scalar,
        dkg_public_key: G1,
    ) -> Self {
        InMemoryNodeInfoCache {
            private_key: vec![],
            id_address,
            node_rpc_endpoint,
            controller_rpc_endpoint,
            dkg_private_key: Some(dkg_private_key),
            dkg_public_key: Some(dkg_public_key),
        }
    }
}

impl NodeInfoFetcher for InMemoryNodeInfoCache {
    fn get_private_key(&self) -> &[u8] {
        &self.private_key
    }

    fn get_id_address(&self) -> &str {
        &self.id_address
    }

    fn get_node_rpc_endpoint(&self) -> &str {
        &self.node_rpc_endpoint
    }

    fn get_controller_rpc_endpoint(&self) -> &str {
        &self.controller_rpc_endpoint
    }

    fn get_dkg_private_key(&self) -> NodeResult<&Scalar> {
        self.dkg_private_key.as_ref().ok_or(NodeError::NoDKGKeyPair)
    }

    fn get_dkg_public_key(&self) -> NodeResult<&G1> {
        self.dkg_public_key.as_ref().ok_or(NodeError::NoDKGKeyPair)
    }
}

// TODO split by use case
pub trait GroupInfoUpdater {
    fn save_task_info(&mut self, self_index: usize, task: DKGTask) -> NodeResult<()>;

    fn save_output(
        &mut self,
        index: usize,
        epoch: usize,
        output: DKGOutput<Curve>,
    ) -> NodeResult<(G1, G1, Vec<String>)>;

    fn save_committers(
        &mut self,
        index: usize,
        epoch: usize,
        committer_indices: Vec<String>,
    ) -> NodeResult<()>;
}

pub trait GroupInfoFetcher {
    fn get_index(&self) -> NodeResult<usize>;

    fn get_epoch(&self) -> NodeResult<usize>;

    fn get_size(&self) -> NodeResult<usize>;

    fn get_threshold(&self) -> NodeResult<usize>;

    fn get_state(&self) -> NodeResult<bool>;

    fn get_public_key(&self) -> NodeResult<&G1>;

    fn get_secret_share(&self) -> NodeResult<&Share<Scalar>>;

    fn get_member(&self, id_address: &str) -> NodeResult<&Member>;

    fn get_committers(&self) -> NodeResult<Vec<&str>>;

    fn get_dkg_start_block_height(&self) -> NodeResult<usize>;

    fn is_committer(&self, id_address: &str) -> NodeResult<bool>;
}

#[derive(Default)]
pub struct InMemoryGroupInfoCache {
    share: Option<Share<Scalar>>,

    group: Group,

    self_index: usize,

    dkg_start_block_height: usize,
}

impl InMemoryGroupInfoCache {
    pub fn new() -> Self {
        let group: Group = Group::new();

        InMemoryGroupInfoCache {
            group,
            share: None,
            self_index: 0,
            dkg_start_block_height: 0,
        }
    }

    fn only_has_group_task(&self) -> NodeResult<()> {
        if self.group.index == 0 {
            return Err(NodeError::NoGroupTask);
        }

        Ok(())
    }
}

impl GroupInfoUpdater for InMemoryGroupInfoCache {
    fn save_task_info(&mut self, self_index: usize, task: DKGTask) -> NodeResult<()> {
        self.self_index = self_index;

        self.group.index = task.group_index;

        self.group.epoch = task.epoch;

        self.group.size = task.size;

        self.group.threshold = task.threshold;

        self.group.public_key = None;

        self.group.state = false;

        self.group.members.clear();

        self.group.committers.clear();

        task.members.iter().for_each(|(address, index)| {
            let member = Member {
                index: *index,
                id_address: address.to_string(),
                rpc_endpint: None,
                partial_public_key: None,
            };
            self.group.members.insert(address.to_string(), member);
        });

        Ok(())
    }

    fn save_output(
        &mut self,
        index: usize,
        epoch: usize,
        output: DKGOutput<Curve>,
    ) -> NodeResult<(G1, G1, Vec<String>)> {
        self.only_has_group_task()?;

        if self.group.index != index {
            return Err(NodeError::GroupIndexObsolete(self.group.index));
        }

        if self.group.epoch != epoch {
            return Err(NodeError::GroupEpochObsolete(self.group.epoch));
        }

        if self.group.state {
            return Err(NodeError::GroupAlreadyReady);
        }

        self.share = Some(output.share);

        // every member index is started from 0
        let qualified_node_indices = output
            .qual
            .nodes
            .iter()
            .map(|node| node.id() as usize)
            .collect::<Vec<_>>();

        self.group.size = qualified_node_indices.len();

        let disqualified_nodes = self
            .group
            .members
            .iter()
            .filter(|(_, member)| !qualified_node_indices.contains(&member.index))
            .map(|(id_address, _)| id_address.to_string())
            .collect::<Vec<_>>();

        self.group
            .members
            .retain(|node, _| !disqualified_nodes.contains(node));

        let public_key = *output.public.public_key();

        self.group.public_key = Some(public_key);

        let mut partial_public_key = G1::new();

        for (_, member) in self.group.members.iter_mut() {
            if let Some(node) = output
                .qual
                .nodes
                .iter()
                .find(|node| member.index == node.id() as usize)
            {
                if let Some(rpc_endpoint) = node.get_rpc_endpoint() {
                    member.rpc_endpint = Some(rpc_endpoint.to_string());
                }
            }

            member.partial_public_key = Some(output.public.eval(member.index as u32).value);

            // println!(
            //     "member index: {}, partial_public_key: {:?}",
            //     member.index, member.partial_public_key
            // );

            if self.self_index == member.index {
                partial_public_key = member.partial_public_key.unwrap();
            }
        }

        Ok((public_key, partial_public_key, disqualified_nodes))
    }

    fn save_committers(
        &mut self,
        index: usize,
        epoch: usize,
        committer_indices: Vec<String>,
    ) -> NodeResult<()> {
        self.only_has_group_task()?;

        if self.group.index != index {
            return Err(NodeError::GroupIndexObsolete(self.group.index));
        }

        if self.group.epoch != epoch {
            return Err(NodeError::GroupEpochObsolete(self.group.epoch));
        }

        if self.group.state {
            return Err(NodeError::GroupAlreadyReady);
        }

        self.group.committers = committer_indices;

        self.group.state = true;

        Ok(())
    }
}

impl GroupInfoFetcher for InMemoryGroupInfoCache {
    fn get_index(&self) -> NodeResult<usize> {
        self.only_has_group_task()?;

        Ok(self.group.index)
    }

    fn get_epoch(&self) -> NodeResult<usize> {
        self.only_has_group_task()?;

        Ok(self.group.epoch)
    }

    fn get_size(&self) -> NodeResult<usize> {
        self.only_has_group_task()?;

        Ok(self.group.size)
    }

    fn get_threshold(&self) -> NodeResult<usize> {
        self.only_has_group_task()?;

        Ok(self.group.threshold)
    }

    fn get_state(&self) -> NodeResult<bool> {
        self.only_has_group_task()?;

        Ok(self.group.state)
    }

    fn get_public_key(&self) -> NodeResult<&G1> {
        self.only_has_group_task()?;

        self.group
            .public_key
            .as_ref()
            .ok_or(NodeError::GroupNotExisted)
    }

    fn get_secret_share(&self) -> NodeResult<&Share<Scalar>> {
        self.only_has_group_task()?;

        self.share.as_ref().ok_or(NodeError::GroupNotReady)
    }

    fn get_member(&self, id_address: &str) -> NodeResult<&Member> {
        self.only_has_group_task()?;

        self.group
            .members
            .get(id_address)
            .ok_or(NodeError::GroupNotExisted)
    }

    fn get_committers(&self) -> NodeResult<Vec<&str>> {
        self.only_has_group_task()?;

        Ok(self
            .group
            .committers
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>())
    }

    fn get_dkg_start_block_height(&self) -> NodeResult<usize> {
        self.only_has_group_task()?;

        Ok(self.dkg_start_block_height)
    }

    fn is_committer(&self, id_address: &str) -> NodeResult<bool> {
        self.only_has_group_task()?;

        Ok(self.group.committers.contains(&id_address.to_string()))
    }
}

pub trait BLSTasksFetcher {
    fn contains(&self, task_index: usize) -> bool;

    fn get(&self, task_index: usize) -> Option<&SignatureTask>;

    fn is_handled(&self, task_index: usize) -> bool;
}

pub trait BLSTasksUpdater {
    fn add(&mut self, task: SignatureTask) -> NodeResult<()>;

    fn check_and_get_available_tasks(
        &mut self,
        current_block_height: usize,
        current_group_index: usize,
    ) -> Vec<SignatureTask>;
}

#[derive(Default)]
pub struct InMemoryBLSTasksQueue {
    bls_tasks: Vec<(SignatureTask, bool)>,
}

impl InMemoryBLSTasksQueue {
    pub fn new() -> Self {
        InMemoryBLSTasksQueue {
            bls_tasks: Vec::new(),
        }
    }
}

impl BLSTasksFetcher for InMemoryBLSTasksQueue {
    fn contains(&self, task_index: usize) -> bool {
        self.bls_tasks
            .iter()
            .any(|(task, _)| task.index == task_index)
    }

    fn get(&self, task_index: usize) -> Option<&SignatureTask> {
        self.bls_tasks.get(task_index).map(|(task, _)| task)
    }

    fn is_handled(&self, task_index: usize) -> bool {
        *self
            .bls_tasks
            .get(task_index)
            .map(|(_, state)| state)
            .or(Some(&false))
            .unwrap()
    }
}

impl BLSTasksUpdater for InMemoryBLSTasksQueue {
    fn add(&mut self, task: SignatureTask) -> NodeResult<()> {
        self.bls_tasks.push((task, false));
        Ok(())
    }

    fn check_and_get_available_tasks(
        &mut self,
        current_block_height: usize,
        current_group_index: usize,
    ) -> Vec<SignatureTask> {
        let available_tasks = self
            .bls_tasks
            .iter_mut()
            .filter(|(_, state)| !state)
            .filter(|(bls_task, _)| {
                bls_task.group_index == current_group_index
                    || current_block_height > bls_task.assignment_block_height + 10
            })
            .map(|e| {
                e.1 = true;
                e.0.clone()
            })
            .collect::<Vec<_>>();

        available_tasks
    }
}

pub trait SignatureResultCacheFetcher {
    fn contains(&self, signature_index: usize) -> bool;
}

pub trait SignatureResultCacheUpdater {
    fn get_ready_to_commit_signatures(&mut self) -> Vec<SignatureResultCache>;

    fn get(&mut self, signature_index: usize) -> NodeResult<&mut SignatureResultCache>;

    fn add(
        &mut self,
        group_index: usize,
        signature_index: usize,
        threshold: usize,
    ) -> NodeResult<bool>;

    fn remove(&mut self, signature_index: usize) -> NodeResult<bool>;

    fn add_partial_signature(
        &mut self,
        signature_index: usize,
        member_address: String,
        partial_signature: Vec<u8>,
    ) -> NodeResult<bool>;
}

#[derive(Default)]
pub struct InMemorySignatureResultCache {
    signature_result_caches: HashMap<usize, SignatureResultCache>,
}

impl InMemorySignatureResultCache {
    pub fn new() -> Self {
        InMemorySignatureResultCache {
            signature_result_caches: HashMap::new(),
        }
    }
}

#[derive(Clone)]
pub struct SignatureResultCache {
    pub group_index: usize,
    pub signature_index: usize,
    pub threshold: usize,
    pub partial_signatures: HashMap<String, Vec<u8>>,
}

impl SignatureResultCacheFetcher for InMemorySignatureResultCache {
    fn contains(&self, signature_index: usize) -> bool {
        self.signature_result_caches.contains_key(&signature_index)
    }
}

impl SignatureResultCacheUpdater for InMemorySignatureResultCache {
    fn get(&mut self, signature_index: usize) -> NodeResult<&mut SignatureResultCache> {
        let cache = self
            .signature_result_caches
            .get_mut(&signature_index)
            .ok_or(NodeError::CommitterCacheNotExisted)?;
        Ok(cache)
    }

    fn add(
        &mut self,
        group_index: usize,
        signature_index: usize,
        threshold: usize,
    ) -> NodeResult<bool> {
        let signature_result_cache = SignatureResultCache {
            group_index,
            signature_index,
            threshold,
            partial_signatures: HashMap::new(),
        };

        self.signature_result_caches
            .insert(signature_index, signature_result_cache);

        Ok(true)
    }

    fn remove(&mut self, signature_index: usize) -> NodeResult<bool> {
        self.signature_result_caches.remove(&signature_index);

        Ok(true)
    }

    fn add_partial_signature(
        &mut self,
        signature_index: usize,
        member_address: String,
        partial_signature: Vec<u8>,
    ) -> NodeResult<bool> {
        let signature_result_cache = self
            .signature_result_caches
            .get_mut(&signature_index)
            .ok_or(NodeError::CommitterCacheNotExisted)?;

        signature_result_cache
            .partial_signatures
            .insert(member_address, partial_signature);

        Ok(true)
    }

    fn get_ready_to_commit_signatures(&mut self) -> Vec<SignatureResultCache> {
        self.signature_result_caches
            .values_mut()
            .filter(|e| e.partial_signatures.len() >= e.threshold)
            .map(|e| e.clone())
            .collect::<Vec<_>>()
    }
}
