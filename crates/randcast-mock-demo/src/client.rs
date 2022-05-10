use parking_lot::RwLock;
use rand::prelude::ThreadRng;
use randcast_mock_demo::node::cache::{
    InMemoryBlockInfoCache, InMemoryGroupInfoCache, NodeInfoFetcher,
};
use randcast_mock_demo::node::client::ControllerTransactions;
use randcast_mock_demo::node::monitor::{
    BlockListener, MockBlockListener, MockStartingGroupingListener, StartingGroupingListener,
};
use randcast_mock_demo::node::{cache::InMemoryNodeInfoCache, client::MockControllerClient};
use std::env;
use std::sync::Arc;
use threshold_bls::schemes::bls12_381::G1Scheme;
use threshold_bls::sig::Scheme;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    static RNG_FN: fn() -> ThreadRng = rand::thread_rng;

    let mut args = env::args();

    args.next();

    let id_address = match args.next() {
        Some(arg) => arg,
        None => panic!("Didn't get a id_address string"),
    };

    println!("id_address: {}", id_address);

    let rng = &mut rand::thread_rng();

    let (private_key, public_key) = G1Scheme::keypair(rng);

    println!("private_key: {}", private_key);
    println!("public_key: {}", public_key);
    println!("-------------------------------------------------------");

    let node_cache = InMemoryNodeInfoCache::new(id_address.clone(), private_key, public_key);

    let group_cache = InMemoryGroupInfoCache::new();

    let block_cache = InMemoryBlockInfoCache::new();

    let controller_address = String::from("http://[::1]:50052");

    let mut client =
        MockControllerClient::new(controller_address, node_cache.get_id_address().to_string())
            .await?;

    client
        .node_register(bincode::serialize(&public_key).unwrap())
        .await?;

    let node_cache_ref = Arc::new(RwLock::new(node_cache));

    let group_cache_ref = Arc::new(RwLock::new(group_cache));

    let block_cache_ref = Arc::new(RwLock::new(block_cache));

    let grouping_listener = MockStartingGroupingListener::new(
        RNG_FN,
        block_cache_ref.clone(),
        node_cache_ref,
        group_cache_ref,
    );

    let grouping_listener_task = tokio::spawn(async move {
        if let Err(e) = grouping_listener.start().await {
            println!("{:?}", e);
        };
    });

    let block_listener = MockBlockListener::new(block_cache_ref);

    let block_listener_task = tokio::spawn(async move {
        if let Err(e) = block_listener.start().await {
            println!("{:?}", e);
        };
    });

    grouping_listener_task.await?;

    block_listener_task.await?;

    Ok(())
}
