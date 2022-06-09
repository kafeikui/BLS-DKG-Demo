use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use threshold_bls::curve::bls12381::G1;

pub trait Task {
    fn index(&self) -> usize;
}

pub struct BLSTask<T> {
    pub task: T,
    pub state: bool,
}

impl Task for SignatureTask {
    fn index(&self) -> usize {
        self.index
    }
}

impl Task for GroupRelayTask {
    fn index(&self) -> usize {
        self.controller_global_epoch
    }
}

impl Task for GroupRelayConfirmationTask {
    fn index(&self) -> usize {
        self.index
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

#[derive(Clone)]
pub struct GroupRelayTask {
    pub controller_global_epoch: usize,
    pub relayed_group_index: usize,
    pub relayed_group_epoch: usize,
    pub assignment_block_height: usize,
}

#[derive(Clone)]
pub struct GroupRelayConfirmationTask {
    pub index: usize,
    pub group_relay_cache_index: usize,
    pub relayed_group_index: usize,
    pub relayed_group_epoch: usize,
    pub relayer_group_index: usize,
    pub assignment_block_height: usize,
}

#[derive(Default, Clone, Serialize, Deserialize)]
pub struct Group {
    pub index: usize,
    pub epoch: usize,
    pub size: usize,
    pub threshold: usize,
    pub state: bool,
    pub public_key: Option<G1>,
    pub members: HashMap<String, Member>,
    pub committers: Vec<String>,
}

impl Group {
    pub fn new() -> Group {
        Group {
            index: 0,
            epoch: 0,
            size: 0,
            threshold: 0,
            state: false,
            public_key: None,
            members: HashMap::new(),
            committers: vec![],
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Member {
    pub index: usize,
    pub id_address: String,
    pub rpc_endpint: Option<String>,
    pub partial_public_key: Option<G1>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct GroupRelayConfirmation {
    pub group: Group,
    pub status: Status,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Hash, Eq)]
pub enum Status {
    Success,
    Complaint,
}

impl From<bool> for Status {
    fn from(b: bool) -> Self {
        if b {
            Status::Success
        } else {
            Status::Complaint
        }
    }
}

impl Status {
    pub(crate) fn is_success(self) -> bool {
        match self {
            Status::Success => true,
            Status::Complaint => false,
        }
    }
}

pub enum GroupRelayConfirmationTaskState {
    NotExisted,
    Available,
    Invalid,
}

impl GroupRelayConfirmationTaskState {
    pub(crate) fn to_i32(self) -> i32 {
        match self {
            GroupRelayConfirmationTaskState::NotExisted => 0,
            GroupRelayConfirmationTaskState::Available => 1,
            GroupRelayConfirmationTaskState::Invalid => 2,
        }
    }
}

impl From<i32> for GroupRelayConfirmationTaskState {
    fn from(b: i32) -> Self {
        match b {
            1 => GroupRelayConfirmationTaskState::Available,
            2 => GroupRelayConfirmationTaskState::Invalid,
            _ => GroupRelayConfirmationTaskState::NotExisted,
        }
    }
}

pub enum TaskType {
    Randomness,
    GroupRelay,
    GroupRelayConfirmation,
}

impl TaskType {
    pub(crate) fn to_i32(self) -> i32 {
        match self {
            TaskType::Randomness => 0,
            TaskType::GroupRelay => 1,
            TaskType::GroupRelayConfirmation => 2,
        }
    }
}

impl From<i32> for TaskType {
    fn from(b: i32) -> Self {
        match b {
            1 => TaskType::GroupRelay,
            2 => TaskType::GroupRelayConfirmation,
            _ => TaskType::Randomness,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    controller_endpoint: String,
    adapters: Vec<Adapter>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Adapter {
    name: String,
    endpoint: String,
}
