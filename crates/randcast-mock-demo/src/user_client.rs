use randcast_mock_demo::node::client::MockControllerClient;
use randcast_mock_demo::node::client::{ControllerTransactions, ControllerViews};
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = env::args();

    args.next();

    let id_address = match args.next() {
        Some(arg) => arg,
        None => panic!("Didn't get an id_address string"),
    };

    let controller_rpc_endpoint = match args.next() {
        Some(arg) => arg,
        None => panic!("Didn't get a controller rpc endpoint string"),
    };

    let instruction = match args.next() {
        Some(arg) => arg,
        None => panic!("Didn't get in instruction string"),
    };

    let mut client = MockControllerClient::new(controller_rpc_endpoint, id_address).await?;

    if instruction == "request" {
        client.request_randomness("asdasdas").await?;

        println!("request randomness successfully");
    } else if instruction == "last_output" {
        let res = client.get_last_output().await?;

        println!("last_randomness_output: {}", res);
    }

    Ok(())
}
