use dkg_core::primitives::minimum_threshold;
use paired::bls12_381::G1;
use std::cmp::max;
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use threshold_bls::schemes::bls12_381::G1Scheme as SigScheme;
use threshold_bls::sig::SignatureScheme;

pub const REWARD_PER_SIGNATURE: usize = 50;

pub const COMMITTER_REWARD_PER_SIGNATURE: usize = 100;

pub const COMMITTER_PENALTY_PER_SIGNATURE: usize = 1000;

pub const CHALLENGE_REWARD_PER_SIGNATURE: usize = 300;

pub const DEFAULT_MINIMUM_THRESHOLD: usize = 3;

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
}

impl Controller {
    pub fn new(initial_entropy: u64) -> Self {
        Controller {
            block_height: 0,
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
        }
    }
}

pub struct Node {
    pub id_address: String,
    pub id_public_key: Vec<u8>,
    pub endpoint: String,
    pub reward_address: String,
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
    index: usize,
    id_address: String,
    partial_public_key: Vec<u8>,
}

#[derive(Clone)]
pub struct CommitCache {
    commit_result: CommitResult,
    partial_public_key: Vec<u8>,
}

#[derive(Hash, Clone)]
pub struct CommitResult {
    group_epoch: usize,
    public_key: Vec<u8>,
    disqualified_nodes: Vec<String>,
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
}

pub struct SignatureReward {
    signature_task: SignatureTask,
    committer: String,
    group: Group,
    partial_signatures: HashMap<String, Vec<u8>>,
}

trait Internal {
    fn freeze_node(&mut self, id_address: &str, pending_until_block: usize);

    fn calculate_hash<T: Hash>(t: &T) -> u64;
}

pub trait MockHelper {
    fn emit_dkg_task(&self) -> &DKGTask;

    fn emit_signature_task(&self) -> &SignatureTask;

    fn mine(&mut self, block_number: usize);
}

pub trait Transactions {
    fn node_register(
        &mut self,
        id_address: String,
        id_public_key: Vec<u8>,
        endpoint: String,
        reward_address: String,
    ) -> bool;

    fn node_quit(&mut self, id_address: String);

    fn node_activate(&mut self, id_address: String);

    fn redeem(&mut self, id_address: String);

    fn claim(&mut self, id_address: String);

    fn commit_dkg(
        &mut self,
        id_address: String,
        group_index: usize,
        group_epoch: usize,
        public_key: Vec<u8>,
        partial_public_key: Vec<u8>,
        disqualified_nodes: Vec<String>,
    ) -> bool;

    fn request(&mut self, message: &str) -> bool;

    fn fulfill(
        &mut self,
        id_address: String,
        signature_index: usize,
        signature: Vec<u8>,
        partial_signatures: HashMap<String, Vec<u8>>,
    ) -> bool;

    fn challenge_reward(&mut self, id_address: String, signature_index: usize) -> bool;
}

pub trait Views {
    fn get_last_output(&self) -> u64;

    fn get_node(&self, id_address: String) -> &Node;

    fn get_group(&self, index: usize) -> &Group;

    fn valid_group_indices(&self) -> Vec<usize>;

    fn pending_signature_tasks(&self) -> Vec<&SignatureTask>;

    fn verifiable_signature_rewards(&self) -> Vec<&SignatureReward>;
}

impl Internal for Controller {
    fn freeze_node(&mut self, id_address: &str, pending_until_block: usize) {
        let node = self.nodes.get_mut(id_address).unwrap();
        node.state = false;
        node.pending_until_block = pending_until_block;
        // regroup which this node belongs to
        todo!()
    }

    fn calculate_hash<T: Hash>(t: &T) -> u64 {
        let mut s = DefaultHasher::new();
        t.hash(&mut s);
        s.finish()
    }
}

impl MockHelper for Controller {
    fn emit_dkg_task(&self) -> &DKGTask {
        &self.dkg_task.as_ref().unwrap()
    }

    fn emit_signature_task(&self) -> &SignatureTask {
        &self.signature_task.as_ref().unwrap()
    }

    fn mine(&mut self, block_number: usize) {
        self.block_height += block_number;
    }
}

impl Transactions for Controller {
    fn node_register(
        &mut self,
        id_address: String,
        id_public_key: Vec<u8>,
        endpoint: String,
        reward_address: String,
    ) -> bool {
        if self.nodes.contains_key(&id_address) {
            return false;
        }

        // mock: staking

        let node = Node {
            id_address: id_address.clone(),
            id_public_key,
            endpoint,
            reward_address,
            state: true,
            pending_until_block: 0,
            staking: 50000,
        };

        self.nodes.insert(id_address.clone(), node);

        self.rewards.insert(id_address.clone(), 0);

        // TODO: now supports single group only
        if self.groups.is_empty() {
            let group = Group {
                index: 1,
                epoch: 0,
                capacity: 10,
                size: 0,
                threshold: DEFAULT_MINIMUM_THRESHOLD,
                state: false,
                public_key: vec![],
                members: HashMap::new(),
                committers: vec![],
                commit_cache: HashMap::new(),
            };
            self.groups.insert(1, group);
        }

        let group = self.groups.get_mut(&1).unwrap();

        group.size += 1;

        let member = Member {
            index: group.size,
            id_address: id_address.clone(),
            partial_public_key: vec![],
        };

        group.members.insert(id_address, member);

        let minimum = minimum_threshold(group.size);

        group.threshold = max(DEFAULT_MINIMUM_THRESHOLD, minimum);

        if group.size >= 3 {
            group.epoch += 1;

            let mut members = HashMap::new();

            for (member_id_address, member) in group.members.iter() {
                members.insert(member_id_address.clone(), member.index.clone());
            }

            let dkg_task = DKGTask {
                group_index: group.index,
                epoch: group.epoch,
                size: group.size,
                threshold: group.threshold,
                members,
                assignment_block_height: self.block_height,
            };

            self.dkg_task = Some(dkg_task);
            // self.emit_dkg_task(dkg_task);
        }

        true
    }

    fn node_quit(&mut self, _id_address: String) {
        todo!()
    }

    fn node_activate(&mut self, _id_address: String) {
        todo!()
    }

    fn redeem(&mut self, _id_address: String) {
        todo!()
    }

    fn claim(&mut self, _id_address: String) {
        todo!()
    }

    fn commit_dkg(
        &mut self,
        id_address: String,
        group_index: usize,
        group_epoch: usize,
        public_key: Vec<u8>,
        partial_public_key: Vec<u8>,
        disqualified_nodes: Vec<String>,
    ) -> bool {
        let group = self.groups.get_mut(&group_index).unwrap();

        if !group.members.contains_key(&id_address) || group.epoch != group_epoch {
            return false;
        }

        let commit_result = CommitResult {
            group_epoch,
            public_key,
            disqualified_nodes: disqualified_nodes.clone(),
        };

        let commit_cache = CommitCache {
            commit_result,
            partial_public_key: partial_public_key.clone(),
        };

        // TODO when next group epoch increments, clean commit_cache, committers
        if group.commit_cache.contains_key(&id_address) {
            return false;
        }

        group.commit_cache.insert(id_address.clone(), commit_cache);

        fn get_identical_over_threshold_commitment(
            controller: &Controller,
            group_index: usize,
        ) -> Option<CommitCache> {
            let group = controller.groups.get(&group_index).unwrap();

            let mut map: HashMap<u64, usize> = HashMap::new();

            for commit_cache in group.commit_cache.values() {
                let count = map
                    .entry(Controller::calculate_hash(&commit_cache.commit_result))
                    .or_insert(0);

                *count += 1;

                if *count >= group.threshold {
                    return Some(commit_cache.clone());
                }
            }

            None
        }

        if group.state {
            // it's no good for a qualified node to miscommits here. So far we don't verify this commitment.
            let member = group.members.get_mut(&id_address).unwrap();

            member.partial_public_key = partial_public_key;
        } else {
            match get_identical_over_threshold_commitment(self, group_index) {
                None => {}
                Some(commit_cache) => {
                    let group = self.groups.get_mut(&group_index).unwrap();

                    group.state = true;

                    group.size -= commit_cache.commit_result.disqualified_nodes.len();

                    group.public_key = commit_cache.commit_result.public_key.clone();

                    commit_cache
                        .commit_result
                        .disqualified_nodes
                        .iter()
                        .for_each(|disqualified_id_address| {
                            group.members.remove(disqualified_id_address);
                        });

                    for (id_address, cache) in group.commit_cache.iter_mut() {
                        if !disqualified_nodes.contains(id_address) {
                            let member = group.members.get_mut(id_address).unwrap();

                            member.partial_public_key = cache.partial_public_key.clone();
                        }
                    }

                    // choose 3 committers randomly by last randomness output

                    let hash1 = Controller::calculate_hash(&self.last_output) as usize;

                    let hash2 = Controller::calculate_hash(&hash1) as usize;

                    let hash3 = Controller::calculate_hash(&hash2) as usize;

                    let mut index_member_map: HashMap<usize, String> = HashMap::new();

                    group.members.iter().for_each(|(id_address, member)| {
                        index_member_map.insert(member.index, id_address.clone());
                    });

                    let mut qualified_indices = group
                        .members
                        .values()
                        .map(|member| member.index)
                        .collect::<Vec<_>>();

                    let c1 = map_to_qualified_indices(
                        hash1 % (qualified_indices.len() + 1),
                        &qualified_indices,
                    );

                    qualified_indices.retain(|&x| x != c1);

                    let c2 = map_to_qualified_indices(
                        hash2 % (qualified_indices.len() + 1),
                        &qualified_indices,
                    );

                    qualified_indices.retain(|&x| x != c2);

                    let c3 = map_to_qualified_indices(
                        hash3 % (qualified_indices.len() + 1),
                        &qualified_indices,
                    );

                    group
                        .committers
                        .push(index_member_map.get(&c1).unwrap().clone());

                    group
                        .committers
                        .push(index_member_map.get(&c2).unwrap().clone());

                    group
                        .committers
                        .push(index_member_map.get(&c3).unwrap().clone());

                    fn map_to_qualified_indices(
                        mut index: usize,
                        qualified_indices: &[usize],
                    ) -> usize {
                        let max = qualified_indices.iter().max().unwrap();

                        while !qualified_indices.contains(&index) {
                            index = (index + 1) % (max + 1);
                        }

                        index
                    }
                }
            }
        }

        true
    }

    fn request(&mut self, message: &str) -> bool {
        let valid_group_indices = self.valid_group_indices();

        if valid_group_indices.is_empty() {
            return false;
        }
        // mock: payment for request

        let mut assignment_group_index = self.last_group_index;

        loop {
            assignment_group_index = (assignment_group_index + 1) % (self.groups.len() + 1);

            if valid_group_indices.contains(&assignment_group_index) {
                break;
            }
        }

        self.signature_count += 1;

        let signature_task = SignatureTask {
            index: self.signature_count,
            message: String::from(message),
            group_index: assignment_group_index,
            assignment_block_height: self.block_height,
        };

        self.signature_task = Some(signature_task.clone());
        // self.emit_signature_task(signature_task.clone());

        self.pending_signature_tasks
            .insert(signature_task.index, signature_task);

        self.last_group_index = assignment_group_index;

        true
    }

    fn fulfill(
        &mut self,
        id_address: String,
        signature_index: usize,
        signature: Vec<u8>,
        partial_signatures: HashMap<String, Vec<u8>>,
    ) -> bool {
        if !self.pending_signature_tasks.contains_key(&signature_index) {
            return false;
        }

        let signature_task = self
            .pending_signature_tasks
            .get(&signature_index)
            .unwrap()
            .clone();

        let group = self
            .groups
            .get(&signature_task.group_index)
            .unwrap()
            .clone();

        if !group.committers.contains(&id_address) {
            return false;
        }

        let message = &signature_task.message;

        let group_public_key: G1 = bincode::deserialize(&group.public_key).unwrap();

        match SigScheme::verify(&group_public_key, &message.as_bytes(), &signature) {
            Ok(()) => {}
            Err(_err) => return false,
        }

        let committer = self.nodes.get_mut(&id_address).unwrap();

        let committer_address = committer.id_address.clone();

        let committer_reward = self.rewards.get_mut(&committer.reward_address).unwrap();

        *committer_reward += COMMITTER_REWARD_PER_SIGNATURE;

        partial_signatures.keys().for_each(|member_id_address| {
            let node = self.nodes.get(member_id_address).unwrap();

            let member_reward = self.rewards.get_mut(&node.reward_address).unwrap();

            *member_reward += REWARD_PER_SIGNATURE;
        });

        self.last_output = Controller::calculate_hash(&signature);

        let signature_reward = SignatureReward {
            signature_task,
            committer: committer_address,
            group,
            partial_signatures,
        };

        self.verifiable_signature_rewards
            .insert(signature_index, signature_reward);

        self.pending_signature_tasks.remove(&signature_index);

        true
    }

    fn challenge_reward(&mut self, id_address: String, signature_index: usize) -> bool {
        if !self
            .verifiable_signature_rewards
            .contains_key(&signature_index)
        {
            return false;
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

            let public_key = bincode::deserialize(public_key_as_bytes).unwrap();

            // Note: partial_signature contains participant index
            let res = SigScheme::verify(&public_key, message.as_bytes(), partial_signature);

            match res {
                Ok(()) => {}
                Err(_err) => {
                    committer.staking -= COMMITTER_PENALTY_PER_SIGNATURE;

                    self.freeze_node(committer_address, 0);

                    if !self.rewards.contains_key(&id_address) {
                        self.rewards.insert(id_address.clone(), 0);
                    }

                    let challenger_reward = self.rewards.get_mut(&id_address).unwrap();

                    *challenger_reward += CHALLENGE_REWARD_PER_SIGNATURE;

                    self.verifiable_signature_rewards.remove(&signature_index);

                    return true;
                }
            }
        }

        self.verifiable_signature_rewards.remove(&signature_index);

        false
    }
}

impl Views for Controller {
    fn get_last_output(&self) -> u64 {
        self.last_output
    }

    fn get_node(&self, id_address: String) -> &Node {
        self.nodes.get(&id_address).unwrap()
    }

    fn get_group(&self, index: usize) -> &Group {
        self.groups.get(&index).unwrap()
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

#[cfg(test)]
pub mod tests {

    #[test]
    fn test_mut() {
        let a = 5;
        let mut b = a;
        b = 6;
        println!("{:#?}", a);
        println!("{:#?}", b);
    }
}
