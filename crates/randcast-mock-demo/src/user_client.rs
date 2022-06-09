use randcast_mock_demo::node::controller_client::MockControllerClient;
use randcast_mock_demo::node::controller_client::{ControllerTransactions, ControllerViews};
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
        None => panic!("Didn't get an instruction string"),
    };

    let mut client = MockControllerClient::new(controller_rpc_endpoint, id_address).await?;

    if instruction == "request" {
        let seed = match args.next() {
            Some(arg) => arg,
            None => panic!("Didn't get a seed string"),
        };
        client.request_randomness(&seed).await?;

        println!("request randomness successfully");
    } else if instruction == "last_output" {
        let res = client.get_last_output().await?;

        println!("last_randomness_output: {}", res);
    }

    Ok(())
}
