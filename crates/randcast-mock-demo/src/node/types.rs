use std::collections::HashMap;

use threshold_bls::curve::bls12381::G1;

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

#[derive(Default)]
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

pub struct Member {
    pub index: usize,
    pub id_address: String,
    pub rpc_endpint: Option<String>,
    pub partial_public_key: Option<G1>,
}
