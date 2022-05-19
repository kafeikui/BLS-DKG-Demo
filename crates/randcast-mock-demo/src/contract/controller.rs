use super::coordinator::{
    Coordinator, MockHelper as CoordinatorMockHelper, Transactions as CoordinatorTransactions,
    Views as CoordinatorViews,
};
use super::errors::{ControllerError, ControllerResult};
use dkg_core::primitives::minimum_threshold;
use std::cmp::{max, Ordering};
use std::collections::hash_map::DefaultHasher;
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use threshold_bls::curve::bls12381::G1;
use threshold_bls::poly::Eval;
use threshold_bls::schemes::bls12_381::G1Scheme as SigScheme;
use threshold_bls::sig::SignatureScheme;

pub const NODE_STAKING_AMOUNT: usize = 50000;

pub const REWARD_PER_SIGNATURE: usize = 50;

pub const DISQUALIFIED_NODE_PENALTY: usize = 1000;

pub const COORDINATOR_STATE_TRIGGER_REWARD: usize = 100;

pub const COMMITTER_REWARD_PER_SIGNATURE: usize = 100;

pub const COMMITTER_PENALTY_PER_SIGNATURE: usize = 1000;

pub const CHALLENGE_REWARD_PER_SIGNATURE: usize = 300;

pub const DEFAULT_MINIMUM_THRESHOLD: usize = 3;

pub const DEFAULT_COMMITTERS_SIZE: usize = 3;

pub const DEFAULT_DKG_PHASE_DURATION: usize = 30;

pub const GROUP_MAX_CAPACITY: usize = 10;

pub const EXPECTED_GROUP_SIZE: usize = 5;

pub const PENDING_BLOCK_AFTER_QUIT: usize = 100;

pub const SIGNATURE_TASK_EXCLUSIVE_WINDOW: usize = 10;

pub const SIGNATURE_REWARDS_VALIDATION_WINDOW: usize = 50;

pub struct Controller {
    pub block_height: usize,
    pub epoch: usize,
    pub signature_count: usize,
    pub last_output: u64,
    pub last_group_index: usize,
    groups: HashMap<usize, Group>,
    nodes: HashMap<String, Node>,
    pub rewards: HashMap<String, usize>,
    pending_signature_tasks: HashMap<usize, SignatureTask>,
    verifiable_signature_rewards: HashMap<usize, SignatureReward>,
    // mock for locally test environment
    dkg_task: Option<DKGTask>,
    signature_task: Option<SignatureTask>,
    pub coordinators: HashMap<usize, (String, Coordinator)>,
    controller_address: String,
}

impl Controller {
    pub fn new(initial_entropy: u64, controller_address: String) -> Self {
        Controller {
            block_height: 100,
            epoch: 1,
            signature_count: 0,
            last_output: initial_entropy,
            last_group_index: 0,
            groups: HashMap::new(),
            nodes: HashMap::new(),
            rewards: HashMap::new(),
            pending_signature_tasks: HashMap::new(),
            verifiable_signature_rewards: HashMap::new(),
            dkg_task: None,
            signature_task: None,
            coordinators: HashMap::new(),
            controller_address,
        }
    }
}

pub struct Node {
    pub id_address: String,
    pub id_public_key: Vec<u8>,
    pub state: bool,
    pub pending_until_block: usize,
    pub staking: usize,
}

#[derive(Clone)]
pub struct Group {
    pub index: usize,
    pub epoch: usize,
    pub capacity: usize,
    pub size: usize,
    pub threshold: usize,
    pub state: bool,
    pub public_key: Vec<u8>,
    pub members: HashMap<String, Member>,
    pub committers: Vec<String>,
    pub commit_cache: HashMap<String, CommitCache>,
}

#[derive(Clone)]
pub struct Member {
    pub index: usize,
    pub id_address: String,
    pub partial_public_key: Vec<u8>,
}

#[derive(Clone)]
pub struct CommitCache {
    commit_result: CommitResult,
    partial_public_key: Vec<u8>,
}

#[derive(Eq, Clone)]
pub struct CommitResult {
    group_epoch: usize,
    public_key: Vec<u8>,
    disqualified_nodes: Vec<String>,
}

impl PartialEq for CommitResult {
    fn eq(&self, other: &Self) -> bool {
        self.group_epoch == other.group_epoch
            && self.public_key == other.public_key
            && self.disqualified_nodes == other.disqualified_nodes
    }
}

impl Hash for CommitResult {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.group_epoch.hash(state);
        self.public_key.hash(state);
        self.disqualified_nodes.hash(state);
    }
}

#[derive(Clone)]
pub struct SignatureTask {
    pub index: usize,
    pub message: String,
    pub group_index: usize,
    pub assignment_block_height: usize,
}

#[derive(Clone)]
pub struct DKGTask {
    pub group_index: usize,
    pub epoch: usize,
    pub size: usize,
    pub threshold: usize,
    pub members: HashMap<String, usize>,
    pub assignment_block_height: usize,
    pub coordinator_address: String,
}

pub struct SignatureReward {
    signature_task: SignatureTask,
    expiration_block_height: usize,
    committer: String,
    group: Group,
    partial_signatures: HashMap<String, Vec<u8>>,
}

trait Internal {
    fn get_strictly_majority_identical_commitment_result(
        &self,
        group_index: usize,
    ) -> (Option<CommitResult>, Vec<String>);

    fn node_join(&mut self, id_address: String) -> ControllerResult<bool>;

    fn find_available_group(&mut self) -> (usize, bool);

    fn add_group(&mut self) -> usize;

    fn rebalance_group(
        &mut self,
        group_a_index: usize,
        group_b_index: usize,
    ) -> ControllerResult<bool>;

    fn add_to_group(
        &mut self,
        node_id_address: String,
        group_index: usize,
        emit_event_instantly: bool,
    ) -> ControllerResult<()>;

    fn remove_from_group(
        &mut self,
        node_id_address: &str,
        group_index: usize,
        emit_event_instantly: bool,
    ) -> ControllerResult<bool>;

    fn emit_group_event(&mut self, group_index: usize) -> ControllerResult<()>;

    fn slash_node(
        &mut self,
        id_address: &str,
        staking_penalty: usize,
        pending_block: usize,
        handle_group: bool,
    ) -> ControllerResult<()>;

    fn freeze_node(
        &mut self,
        id_address: &str,
        pending_block: usize,
        handle_group: bool,
    ) -> ControllerResult<()>;

    fn calculate_hash<T: Hash>(t: &T) -> u64;
}

pub trait MockHelper {
    fn emit_dkg_task(&self) -> ControllerResult<DKGTask>;

    fn emit_signature_task(&self) -> ControllerResult<SignatureTask>;

    fn mine(&mut self, block_number: usize) -> ControllerResult<usize>;
}

pub trait Transactions {
    fn node_register(&mut self, id_address: String, id_public_key: Vec<u8>)
        -> ControllerResult<()>;

    fn node_activate(&mut self, id_address: String) -> ControllerResult<()>;

    fn node_quit(&mut self, id_address: &str) -> ControllerResult<()>;

    fn claim(
        &mut self,
        id_address: &str,
        reward_address: &str,
        token_requested: usize,
    ) -> ControllerResult<()>;

    fn commit_dkg(
        &mut self,
        id_address: String,
        group_index: usize,
        group_epoch: usize,
        public_key: Vec<u8>,
        partial_public_key: Vec<u8>,
        disqualified_nodes: Vec<String>,
    ) -> ControllerResult<()>;

    fn check_dkg_state(&mut self, id_address: &str, group_index: usize) -> ControllerResult<()>;

    fn request_randomness(&mut self, message: &str) -> ControllerResult<()>;

    fn fulfill_randomness(
        &mut self,
        id_address: &str,
        group_index: usize,
        signature_index: usize,
        signature: Vec<u8>,
        partial_signatures: HashMap<String, Vec<u8>>,
    ) -> ControllerResult<()>;

    fn challenge_verifiable_reward(
        &mut self,
        id_address: &str,
        signature_index: usize,
    ) -> ControllerResult<()>;

    fn check_verifiable_rewards_expiration(&mut self) -> ControllerResult<()>;
}

pub trait Views {
    fn get_last_output(&self) -> u64;

    fn get_node(&self, id_address: &str) -> &Node;

    fn get_group(&self, index: usize) -> &Group;

    fn get_signature_task_completion_state(&self, index: usize) -> bool;

    fn valid_group_indices(&self) -> Vec<usize>;

    fn pending_signature_tasks(&self) -> Vec<&SignatureTask>;

    fn verifiable_signature_rewards(&self) -> Vec<&SignatureReward>;
}

impl Internal for Controller {
    fn get_strictly_majority_identical_commitment_result(
        &self,
        group_index: usize,
    ) -> (Option<CommitResult>, Vec<String>) {
        let group = self.groups.get(&group_index).unwrap();

        let mut map: HashMap<CommitResult, Vec<String>> = HashMap::new();

        for (member, commit_cache) in group.commit_cache.iter() {
            let majority_members = map
                .entry(commit_cache.commit_result.clone())
                .or_insert(vec![]);

            majority_members.push(member.to_string());
        }

        let (r, majority_members, is_strictly_majority) =
            map.into_iter().fold((None, vec![], false), |acc, r| {
                match r.1.len().cmp(&acc.1.len()) {
                    Ordering::Greater => (Some(r.0), r.1, true),
                    Ordering::Equal => (acc.0, acc.1, false),
                    _ => acc,
                }
            });

        if is_strictly_majority {
            return (r, majority_members);
        }

        (None, vec![])
    }

    fn node_join(&mut self, id_address: String) -> ControllerResult<bool> {
        let (group_index, need_rebalance) = self.find_available_group();

        self.add_to_group(id_address, group_index, true)?;

        let group_indices = self
            .groups
            .keys()
            .copied()
            .filter(|i| *i != group_index)
            .collect::<Vec<_>>();

        if need_rebalance {
            group_indices.iter().try_for_each(|index| {
                if let Ok(true) = self.rebalance_group(*index, group_index) {
                    return None;
                }
                Some(())
            });
        }

        Ok(true)
    }

    fn emit_group_event(&mut self, group_index: usize) -> ControllerResult<()> {
        self.epoch += 1;

        let group = self.groups.get_mut(&group_index).unwrap();

        group.epoch += 1;

        group.commit_cache = HashMap::new();

        group.committers = vec![];

        let group = self.groups.get(&group_index).unwrap();

        // create coordinator instance
        let mut coordinator =
            Coordinator::new(group.epoch, group.threshold, DEFAULT_DKG_PHASE_DURATION);

        let mut members = group
            .members
            .values()
            .map(|m| {
                let public_key = self.nodes.get(&m.id_address).unwrap().id_public_key.clone();
                (m.id_address.clone(), m.index, public_key)
            })
            .collect::<Vec<_>>();

        members.sort_by(|a, b| a.1.cmp(&b.1));

        coordinator.start(self.block_height, members)?;

        self.coordinators.insert(
            group.index,
            (format!("0xcoordinator{}", group.index), coordinator),
        );

        // emit event
        let mut members = HashMap::new();

        for (member_id_address, member) in group.members.iter() {
            members.insert(member_id_address.clone(), member.index);
        }

        let dkg_task = DKGTask {
            group_index: group.index,
            epoch: group.epoch,
            size: group.size,
            threshold: group.threshold,
            members,
            assignment_block_height: self.block_height,
            coordinator_address: self.controller_address.clone(),
        };

        self.dkg_task = Some(dkg_task);
        // self.emit_dkg_task(dkg_task);

        Ok(())
    }

    fn find_available_group(&mut self) -> (usize, bool) {
        if self.groups.is_empty() {
            return (self.add_group(), false);
        }

        let (index_of_min_size, min_size) = self
            .groups
            .values()
            .map(|g| (g.index, g.size))
            .min_by(|x, y| x.1.cmp(&y.1))
            .unwrap();

        let valid_group_count = self.valid_group_indices().len();

        if (valid_group_count < EXPECTED_GROUP_SIZE || min_size == GROUP_MAX_CAPACITY)
            && valid_group_count == self.groups.len()
        {
            return (self.add_group(), true);
        }

        (index_of_min_size, false)
    }

    fn add_group(&mut self) -> usize {
        let group_index = self.groups.len() + 1;

        let group = Group {
            index: group_index,
            epoch: 0,
            capacity: GROUP_MAX_CAPACITY,
            size: 0,
            threshold: DEFAULT_MINIMUM_THRESHOLD,
            state: false,
            public_key: vec![],
            members: HashMap::new(),
            committers: vec![],
            commit_cache: HashMap::new(),
        };

        self.groups.insert(group_index, group);

        group_index
    }

    fn rebalance_group(
        &mut self,
        mut group_a_index: usize,
        mut group_b_index: usize,
    ) -> ControllerResult<bool> {
        let mut group_a = self.groups.get(&group_a_index).unwrap();

        let mut group_b = self.groups.get(&group_b_index).unwrap();

        if group_b.size > group_a.size {
            std::mem::swap(&mut group_a, &mut group_b);

            std::mem::swap(&mut group_a_index, &mut group_b_index);
        }

        let expected_size_to_move = group_a.size - (group_a.size + group_b.size) / 2;

        if group_a.size - expected_size_to_move < DEFAULT_MINIMUM_THRESHOLD {
            return Ok(false);
        }

        let qualified_indices = group_a
            .members
            .values()
            .map(|member| member.index)
            .collect::<Vec<_>>();

        let members_to_move = choose_randomly_from_indices(
            self.last_output as usize,
            &qualified_indices,
            expected_size_to_move,
        );

        let mut index_member_map: HashMap<usize, String> = HashMap::new();

        group_a.members.iter().for_each(|(id_address, member)| {
            index_member_map.insert(member.index, id_address.clone());
        });

        for m in members_to_move.iter() {
            self.remove_from_group(index_member_map.get(m).unwrap(), group_a_index, false)?;

            self.add_to_group(
                index_member_map.get(m).unwrap().clone(),
                group_b_index,
                false,
            )?;
        }

        self.emit_group_event(group_a_index)?;

        self.emit_group_event(group_b_index)?;

        Ok(true)
    }

    fn add_to_group(
        &mut self,
        node_id_address: String,
        group_index: usize,
        emit_event_instantly: bool,
    ) -> ControllerResult<()> {
        let group = self.groups.get_mut(&group_index).unwrap();

        let member = Member {
            index: group.size,
            id_address: node_id_address.clone(),
            partial_public_key: vec![],
        };

        group.size += 1;

        group.members.insert(node_id_address, member);

        let minimum = minimum_threshold(group.size);

        group.threshold = max(DEFAULT_MINIMUM_THRESHOLD, minimum);

        if group.size >= 3 && emit_event_instantly {
            self.emit_group_event(group_index)?;
        }

        Ok(())
    }

    fn remove_from_group(
        &mut self,
        node_id_address: &str,
        group_index: usize,
        emit_event_instantly: bool,
    ) -> ControllerResult<bool> {
        let group = self.groups.get_mut(&group_index).unwrap();

        group.size -= 1;

        group.members.remove(node_id_address);

        let minimum = minimum_threshold(group.size);

        group.threshold = max(DEFAULT_MINIMUM_THRESHOLD, minimum);

        if group.size < 3 {
            group.state = false;

            return Ok(group.size > 0);
        }

        if emit_event_instantly {
            self.emit_group_event(group_index)?;
        }

        Ok(false)
    }

    fn slash_node(
        &mut self,
        id_address: &str,
        staking_penalty: usize,
        pending_block: usize,
        handle_group: bool,
    ) -> ControllerResult<()> {
        let node = self.nodes.get_mut(id_address).unwrap();

        node.staking -= staking_penalty;

        if node.staking < NODE_STAKING_AMOUNT || pending_block > 0 {
            self.freeze_node(id_address, pending_block, handle_group)?;
        }

        Ok(())
    }

    fn freeze_node(
        &mut self,
        id_address: &str,
        pending_block: usize,
        handle_group: bool,
    ) -> ControllerResult<()> {
        if handle_group {
            let belong_to_group = self
                .groups
                .values()
                .find(|g| g.members.contains_key(id_address));

            if let Some(group) = belong_to_group {
                let group_index = group.index;

                let need_rebalance = self.remove_from_group(id_address, group_index, true)?;

                let group_indices = self
                    .groups
                    .keys()
                    .copied()
                    .filter(|i| *i != group_index)
                    .collect::<Vec<_>>();

                if need_rebalance {
                    let rebalance_failure = group_indices.iter().try_for_each(|index| {
                        if let Ok(true) = self.rebalance_group(*index, group_index) {
                            return None;
                        }
                        Some(())
                    });

                    if rebalance_failure.is_some() {
                        let members_left_in_group = self
                            .groups
                            .get(&group_index)
                            .unwrap()
                            .members
                            .keys()
                            .map(|m| m.to_string())
                            .collect::<Vec<_>>();

                        let mut invovled_groups = HashSet::new();

                        for member_address in members_left_in_group.iter() {
                            let (group_index, _) = self.find_available_group();

                            self.add_to_group(member_address.to_string(), group_index, false)?;

                            invovled_groups.insert(group_index);
                        }

                        for index in invovled_groups.iter() {
                            self.emit_group_event(*index)?;
                        }
                    }
                }
            }
        }

        let node = self.nodes.get_mut(id_address).unwrap();

        node.state = false;

        node.pending_until_block = if node.pending_until_block > self.block_height {
            node.pending_until_block + pending_block
        } else {
            self.block_height + pending_block
        };

        Ok(())
    }

    fn calculate_hash<T: Hash>(t: &T) -> u64 {
        let mut s = DefaultHasher::new();
        t.hash(&mut s);
        s.finish()
    }
}

impl MockHelper for Controller {
    fn emit_dkg_task(&self) -> ControllerResult<DKGTask> {
        self.dkg_task
            .clone()
            .ok_or(ControllerError::NoTaskAvailable)
    }

    fn emit_signature_task(&self) -> ControllerResult<SignatureTask> {
        self.signature_task
            .clone()
            .ok_or(ControllerError::NoTaskAvailable)
    }

    fn mine(&mut self, block_number: usize) -> ControllerResult<usize> {
        self.block_height += block_number;

        self.coordinators
            .values_mut()
            .for_each(|(_, c)| c.mine(block_number));

        // println!("controller block_height: {}", self.block_height);

        Ok(self.block_height)
    }
}

impl Transactions for Controller {
    fn node_register(
        &mut self,
        id_address: String,
        id_public_key: Vec<u8>,
    ) -> ControllerResult<()> {
        if self.nodes.contains_key(&id_address) {
            return Err(ControllerError::NodeExisted);
        }
        // mock: initial staking

        let node = Node {
            id_address: id_address.clone(),
            id_public_key,
            state: true,
            pending_until_block: 0,
            staking: NODE_STAKING_AMOUNT,
        };

        self.nodes.insert(id_address.clone(), node);

        self.rewards.insert(id_address.clone(), 0);

        self.node_join(id_address)?;

        Ok(())
    }

    fn node_activate(&mut self, id_address: String) -> ControllerResult<()> {
        if !self.nodes.contains_key(&id_address) {
            return Err(ControllerError::NodeNotExisted);
        }

        let node = self.nodes.get_mut(&id_address).unwrap();

        if node.state {
            return Err(ControllerError::NodeActivated);
        }

        if node.pending_until_block > self.block_height {
            return Err(ControllerError::NodeNotAvailable(node.pending_until_block));
        }

        // mock: fill staking
        node.staking = NODE_STAKING_AMOUNT;

        self.node_join(id_address)?;

        Ok(())
    }

    fn node_quit(&mut self, id_address: &str) -> ControllerResult<()> {
        if !self.nodes.contains_key(id_address) {
            return Err(ControllerError::NodeNotExisted);
        }

        self.check_verifiable_rewards_expiration()?;

        if self
            .verifiable_signature_rewards
            .values()
            .any(|vsr| vsr.committer == *id_address)
        {
            return Err(ControllerError::VerifiableSignatureRewardAsCommitterExisted);
        }

        self.freeze_node(id_address, PENDING_BLOCK_AFTER_QUIT, true)?;

        // mock token redeem

        Ok(())
    }

    fn claim(
        &mut self,
        id_address: &str,
        _reward_address: &str,
        token_amount: usize,
    ) -> ControllerResult<()> {
        if !self.rewards.contains_key(id_address) {
            return Err(ControllerError::RewardRecordNotExisted);
        }

        let actual_amount = self.rewards.get_mut(id_address).unwrap();

        let operate_amount = if *actual_amount >= token_amount {
            token_amount
        } else {
            *actual_amount
        };

        // mock redeem to reward_address

        *actual_amount -= operate_amount;

        Ok(())
    }

    fn commit_dkg(
        &mut self,
        id_address: String,
        group_index: usize,
        group_epoch: usize,
        public_key: Vec<u8>,
        partial_public_key: Vec<u8>,
        disqualified_nodes: Vec<String>,
    ) -> ControllerResult<()> {
        if !self.groups.contains_key(&group_index) {
            return Err(ControllerError::GroupNotExisted);
        }

        bincode::deserialize::<G1>(&public_key)?;

        bincode::deserialize::<G1>(&partial_public_key)?;

        let group = self.groups.get_mut(&group_index).unwrap();

        if !group.members.contains_key(&id_address) {
            return Err(ControllerError::ParticipantNotExisted);
        }

        if group.epoch != group_epoch {
            return Err(ControllerError::GroupEpochObsolete(group.epoch));
        }

        let commit_result = CommitResult {
            group_epoch,
            public_key,
            disqualified_nodes,
        };

        let commit_cache = CommitCache {
            commit_result,
            partial_public_key: partial_public_key.clone(),
        };

        if group.commit_cache.contains_key(&id_address) {
            return Err(ControllerError::CommitCacheExisted);
        }

        group.commit_cache.insert(id_address.clone(), commit_cache);

        if group.state {
            // it's no good for a qualified node to miscommits here. So far we don't verify this commitment.
            let member = group.members.get_mut(&id_address).unwrap();

            member.partial_public_key = partial_public_key;
        } else {
            match self.get_strictly_majority_identical_commitment_result(group_index) {
                (None, _) => {}

                (Some(identical_commit), majority_members) => {
                    let group = self.groups.get_mut(&group_index).unwrap();

                    if majority_members.len() >= group.threshold {
                        group.state = true;

                        group.size -= identical_commit.disqualified_nodes.len();

                        group.public_key = identical_commit.public_key.clone();

                        let disqualified_nodes = identical_commit.disqualified_nodes;

                        for (id_address, cache) in group.commit_cache.iter_mut() {
                            if !disqualified_nodes.contains(id_address) {
                                let member = group.members.get_mut(id_address).unwrap();

                                member.partial_public_key = cache.partial_public_key.clone();
                            }
                        }

                        // choose max(3, threshold) committers randomly by last randomness output
                        let mut index_member_map: HashMap<usize, String> = HashMap::new();

                        group.members.iter().for_each(|(id_address, member)| {
                            index_member_map.insert(member.index, id_address.clone());
                        });

                        let qualified_indices = group
                            .members
                            .values()
                            .map(|member| member.index)
                            .collect::<Vec<_>>();

                        let committer_indices = choose_randomly_from_indices(
                            self.last_output as usize,
                            &qualified_indices,
                            max(DEFAULT_COMMITTERS_SIZE, group.threshold),
                        );

                        committer_indices.iter().for_each(|c| {
                            group
                                .committers
                                .push(index_member_map.get(c).unwrap().clone());
                        });

                        // move out these disqualified_nodes from the group first
                        group
                            .members
                            .retain(|node, _| !disqualified_nodes.contains(node));

                        for disqualified_node in disqualified_nodes {
                            self.slash_node(
                                &disqualified_node,
                                DISQUALIFIED_NODE_PENALTY,
                                0,
                                false,
                            )?;
                        }
                    }
                }
            }
        }

        Ok(())
    }

    fn check_dkg_state(&mut self, id_address: &str, group_index: usize) -> ControllerResult<()> {
        // handles coordinator selfdestruct if reaches DKG timeout, arranges members if fail grouping, and rewards trigger (sender)
        let group = self
            .groups
            .get(&group_index)
            .ok_or(ControllerError::GroupNotExisted)?;

        if group.state {
            return Err(ControllerError::GroupActivated);
        }

        let (_, coordinator) = self
            .coordinators
            .get_mut(&group_index)
            .ok_or(ControllerError::CoordinatorNotExisted)?;

        if coordinator.epoch < group.epoch {
            return Err(ControllerError::CoordinatorEpochObsolete(group.epoch));
        }

        if coordinator.in_phase().is_ok() {
            return Err(ControllerError::CoordinatorNotEnded);
        }

        match self.get_strictly_majority_identical_commitment_result(group_index) {
            (None, _) => {
                let group = self.groups.get_mut(&group_index).unwrap();

                group.size = 0;

                group.threshold = 0;

                let members = group
                    .members
                    .keys()
                    .map(|m| m.to_string())
                    .collect::<Vec<_>>();

                group.members.clear();

                for m in members {
                    self.slash_node(&m, DISQUALIFIED_NODE_PENALTY, 0, false)?;
                }
            }

            (Some(_), majority_members) => {
                let group = self.groups.get_mut(&group_index).unwrap();

                let disqualified_nodes = group
                    .members
                    .keys()
                    .filter(|m| !majority_members.contains(m))
                    .map(|m| m.to_string())
                    .collect::<Vec<_>>();

                group.size -= disqualified_nodes.len();

                group
                    .members
                    .retain(|node, _| !disqualified_nodes.contains(node));

                for (index, disqualified_node) in disqualified_nodes.iter().enumerate() {
                    let handle_group = index == disqualified_node.len() - 1;

                    self.slash_node(
                        disqualified_node,
                        DISQUALIFIED_NODE_PENALTY,
                        0,
                        handle_group,
                    )?;
                }
            }
        }

        if !self.rewards.contains_key(id_address) {
            self.rewards.insert(id_address.to_string(), 0);
        }

        let trigger_reward = self.rewards.get_mut(id_address).unwrap();

        *trigger_reward += COORDINATOR_STATE_TRIGGER_REWARD;

        self.coordinators.remove(&group_index);

        Ok(())
    }

    fn request_randomness(&mut self, message: &str) -> ControllerResult<()> {
        let valid_group_indices = self.valid_group_indices();

        println!("request randomness successfully");

        if valid_group_indices.is_empty() {
            println!("no available group!");
            return Err(ControllerError::NoVaildGroup);
        }
        // mock: payment for request

        let mut assignment_group_index = self.last_group_index;

        loop {
            assignment_group_index = (assignment_group_index + 1) % (self.groups.len() + 1);

            if valid_group_indices.contains(&assignment_group_index) {
                break;
            }
        }

        let signature_task = SignatureTask {
            index: self.signature_count,
            message: format!("{}{}{}", message, &self.block_height, &self.last_output),
            group_index: assignment_group_index,
            assignment_block_height: self.block_height,
        };

        self.signature_count += 1;

        self.signature_task = Some(signature_task.clone());
        // self.emit_signature_task(signature_task.clone());

        self.pending_signature_tasks
            .insert(signature_task.index, signature_task);

        self.last_group_index = assignment_group_index;

        Ok(())
    }

    fn fulfill_randomness(
        &mut self,
        id_address: &str,
        group_index: usize,
        signature_index: usize,
        signature: Vec<u8>,
        partial_signatures: HashMap<String, Vec<u8>>,
    ) -> ControllerResult<()> {
        if !self.pending_signature_tasks.contains_key(&signature_index) {
            return Err(ControllerError::TaskNotFound);
        }

        let signature_task = self
            .pending_signature_tasks
            .get(&signature_index)
            .unwrap()
            .clone();

        if (self.block_height
            <= signature_task.assignment_block_height + SIGNATURE_TASK_EXCLUSIVE_WINDOW)
            && group_index != signature_task.group_index
        {
            return Err(ControllerError::TaskStillExclusive);
        }

        let group = self
            .groups
            .get(&group_index)
            .ok_or(ControllerError::GroupNotExisted)?
            .clone();

        if !group.committers.contains(&id_address.to_string()) {
            return Err(ControllerError::NotFromCommitter);
        }

        let message = &signature_task.message;

        let group_public_key: G1 = bincode::deserialize(&group.public_key)?;

        SigScheme::verify(&group_public_key, message.as_bytes(), &signature)?;

        let committer = self
            .nodes
            .get_mut(id_address)
            .ok_or(ControllerError::NodeNotExisted)?;

        let committer_address = committer.id_address.clone();

        for member_id_address in partial_signatures.keys() {
            if !group.members.contains_key(member_id_address) {
                return Err(ControllerError::ParticipantNotExisted);
            }
        }

        let committer_reward = self
            .rewards
            .get_mut(&committer_address)
            .ok_or(ControllerError::RewardRecordNotExisted)?;

        *committer_reward += COMMITTER_REWARD_PER_SIGNATURE;

        for member_id_address in partial_signatures.keys() {
            let node = self
                .nodes
                .get(member_id_address)
                .ok_or(ControllerError::NodeNotExisted)?;

            let member_reward = self
                .rewards
                .get_mut(&node.id_address)
                .ok_or(ControllerError::RewardRecordNotExisted)?;

            *member_reward += REWARD_PER_SIGNATURE;
        }

        self.last_output = Controller::calculate_hash(&signature);

        let signature_reward = SignatureReward {
            signature_task,
            expiration_block_height: self.block_height + SIGNATURE_REWARDS_VALIDATION_WINDOW,
            committer: committer_address,
            group,
            partial_signatures,
        };

        self.verifiable_signature_rewards
            .insert(signature_index, signature_reward);

        self.pending_signature_tasks.remove(&signature_index);

        Ok(())
    }

    fn challenge_verifiable_reward(
        &mut self,
        id_address: &str,
        signature_index: usize,
    ) -> ControllerResult<()> {
        if !self
            .verifiable_signature_rewards
            .contains_key(&signature_index)
        {
            return Err(ControllerError::VerifiableSignatureRewardNotExisted);
        }

        let signature_reward = self
            .verifiable_signature_rewards
            .get(&signature_index)
            .unwrap();

        let group = &signature_reward.group;

        let committer = self.nodes.get_mut(&signature_reward.committer).unwrap();

        let committer_address = &committer.id_address.clone();

        let message = &signature_reward.signature_task.message;

        // TODO need a BLS-Aggregation Verification instead of loop to save computational fee
        for (member_id_address, partial_signature) in signature_reward.partial_signatures.iter() {
            let public_key_as_bytes = &group
                .members
                .get(member_id_address)
                .unwrap()
                .partial_public_key;

            let public_key = bincode::deserialize(public_key_as_bytes)?;

            // Note: decouple signature value and participant index from partial_signature
            let res = bincode::deserialize(partial_signature)
                .map_err(ControllerError::from)
                .and_then(|partial_signature: Eval<Vec<u8>>| {
                    SigScheme::verify(&public_key, message.as_bytes(), &partial_signature.value)
                        .map_err(ControllerError::from)
                });

            match res {
                Ok(()) => {}
                Err(_err) => {
                    self.slash_node(committer_address, COMMITTER_PENALTY_PER_SIGNATURE, 0, true)?;

                    if !self.rewards.contains_key(id_address) {
                        self.rewards.insert(id_address.to_string(), 0);
                    }

                    let challenger_reward = self.rewards.get_mut(id_address).unwrap();

                    *challenger_reward += CHALLENGE_REWARD_PER_SIGNATURE;

                    self.verifiable_signature_rewards.remove(&signature_index);

                    return Ok(());
                }
            }
        }

        self.verifiable_signature_rewards.remove(&signature_index);

        Err(ControllerError::SignatureRewardVerifiedSuccessfully)
    }

    fn check_verifiable_rewards_expiration(&mut self) -> ControllerResult<()> {
        let current_block_height = self.block_height;

        self.verifiable_signature_rewards
            .retain(|_, vsr| current_block_height <= vsr.expiration_block_height);

        Ok(())
    }
}

impl Views for Controller {
    fn get_last_output(&self) -> u64 {
        self.last_output
    }

    fn get_node(&self, id_address: &str) -> &Node {
        self.nodes.get(id_address).unwrap()
    }

    fn get_group(&self, index: usize) -> &Group {
        self.groups.get(&index).unwrap()
    }

    fn get_signature_task_completion_state(&self, index: usize) -> bool {
        index < self.signature_count && !self.pending_signature_tasks.contains_key(&index)
    }

    fn valid_group_indices(&self) -> Vec<usize> {
        self.groups
            .values()
            .filter(|g| g.state)
            .map(|g| g.index)
            .collect::<Vec<_>>()
    }

    fn pending_signature_tasks(&self) -> Vec<&SignatureTask> {
        self.pending_signature_tasks.values().collect::<Vec<_>>()
    }

    fn verifiable_signature_rewards(&self) -> Vec<&SignatureReward> {
        self.verifiable_signature_rewards
            .values()
            .collect::<Vec<_>>()
    }
}

fn choose_randomly_from_indices(seed: usize, indices: &[usize], mut count: usize) -> Vec<usize> {
    let mut vec = indices.to_vec();

    let mut res: Vec<usize> = Vec::new();

    let mut hash = seed;

    while count > 0 && !vec.is_empty() {
        hash = Controller::calculate_hash(&hash) as usize;

        let index = map_to_qualified_indices(hash % (vec.len() + 1), &vec);

        res.push(index);

        vec.retain(|&x| x != index);

        count -= 1;
    }

    res
}

fn map_to_qualified_indices(mut index: usize, qualified_indices: &[usize]) -> usize {
    let max = qualified_indices.iter().max().unwrap();

    while !qualified_indices.contains(&index) {
        index = (index + 1) % (max + 1);
    }

    index
}

#[cfg(test)]
pub mod tests {
    use super::{Controller, Transactions};

    #[test]
    fn test() {
        let initial_entropy = 0x8762_4875_6548_6346;

        let mut controller = Controller::new(initial_entropy, "0xcontroller_address".to_string());

        let node_address = "0x1";

        controller.rewards.insert(node_address.to_string(), 1000);

        controller.claim(node_address, node_address, 200).unwrap();

        println!("{:?}", controller.rewards.get(node_address));
    }

    #[test]
    fn test2() {
        let vec1 = vec![String::from("232wer3")];
        let vec2 = vec![String::from("232wer3")];
        println!("{}", vec1 == vec2);
    }

    #[test]
    fn test3() {
        let str = String::from("ewrfw");
        let vec = bincode::serialize(&str).unwrap();
        let asda: String = bincode::deserialize(&vec).unwrap();
        println!("{}", asda);
        println!("{}", str == asda);
    }
}
