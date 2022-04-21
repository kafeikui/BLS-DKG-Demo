use super::{
    cache::GroupInfoFetcher,
    client::CoordinatorViews,
    errors::{NodeError, NodeResult},
    types::DKGTask,
};
use crate::node::client::MockCoordinatorClient;
use async_trait::async_trait;
use dkg_core::{
    primitives::{joint_feldman::*, *},
    DKGPhase, Phase2Result,
};
use parking_lot::RwLock;
use rand::RngCore;
use rustc_hex::ToHex;
use std::{io::Write, sync::Arc};
use threshold_bls::{
    curve::bls12381::{Curve, Scalar, G1},
    poly::Idx,
};

#[async_trait]
pub trait DKGCore<F, R> {
    async fn run_dkg(
        &mut self,
        dkg_private_key: Scalar,
        id_address: String,
        task: DKGTask,
        rng: F,
        group_info_fetcher: Arc<RwLock<impl GroupInfoFetcher + Send + Sync + 'async_trait>>,
    ) -> NodeResult<DKGOutput<Curve>>
    where
        R: RngCore,
        F: Fn() -> R + Send + 'async_trait;
}

pub struct MockDKGCore {}

#[async_trait]
impl<F, R> DKGCore<F, R> for MockDKGCore
where
    R: RngCore,
    F: Fn() -> R + Send,
{
    async fn run_dkg(
        &mut self,
        dkg_private_key: Scalar,
        id_address: String,
        task: DKGTask,
        rng: F,
        group_info_fetcher: Arc<RwLock<impl GroupInfoFetcher + Send + Sync + 'async_trait>>,
    ) -> NodeResult<DKGOutput<Curve>>
    where
        F: 'async_trait,
    {
        // TODO
        let coordinator_address = String::from("http://[::1]:50052");

        let mut dkg = MockCoordinatorClient::new(
            coordinator_address,
            id_address,
            task.group_index,
            task.epoch,
        )
        .await?;

        // 1. Generate the keys
        // let (private_key, public_key) = S::keypair(rng);

        // 2. no need to register, just wait for phase1 for now

        // Wait for Phase 1
        wait_for_phase(&mut dkg, 1).await?;
        check_epoch_valid(&task, group_info_fetcher.clone())?;

        // Get the group info
        let group = dkg.get_bls_keys().await?;
        let participants = dkg.get_participants().await?;

        // print some debug info
        println!(
            "Will run DKG with the group listed below and threshold {}",
            group.0
        );
        for (bls_pubkey, address) in group.1.iter().zip(&participants) {
            let key = bls_pubkey.to_hex::<String>();
            println!("{:?} -> {}", address, key)
        }

        // if !clt::confirm(
        //     "\nDoes the above group look good to you?",
        //     false,
        //     "\n",
        //     true,
        // ) {
        //     return Err(anyhow::anyhow!("User rejected group choice."));
        // }

        let nodes = group
            .1
            .into_iter()
            .filter(|pubkey| !pubkey.is_empty()) // skip users that did not register
            .enumerate()
            .map(|(i, pubkey)| {
                let pubkey: G1 = bincode::deserialize(&pubkey)?;
                Ok(Node::<Curve>::new(i as Idx, pubkey))
            })
            .collect::<NodeResult<_>>()?;

        let group = Group {
            threshold: group.0,
            nodes,
        };

        // Instantiate the DKG with the group info
        println!("Calculating and broadcasting our shares...");
        let phase0 = DKG::new(dkg_private_key, group)?;

        // Run Phase 1 and publish to the chain
        let phase1 = phase0.run(&mut dkg, rng).await?;

        // Wait for Phase 2
        wait_for_phase(&mut dkg, 2).await?;
        check_epoch_valid(&task, group_info_fetcher.clone())?;

        // Get the shares
        let shares = dkg.get_shares().await?;
        println!("Got {} shares...", shares.len());
        let shares = parse_bundle(&shares)?;
        println!("Parsed {} shares. Running Phase 2", shares.len());

        let phase2 = phase1.run(&mut dkg, &shares).await?;

        // Get the responses
        let responses = dkg.get_responses().await?;
        println!("Got {} responses...", responses.len());
        let responses = parse_bundle(&responses)?;
        println!("Parsed the responses. Getting result.");

        // Run Phase 2
        let result = match phase2.run(&mut dkg, &responses).await? {
            Phase2Result::Output(out) => Ok(out),
            // Run Phase 3 if Phase 2 errored
            Phase2Result::GoToPhase3(phase3) => {
                println!("There were complaints. Running Phase 3.");
                wait_for_phase(&mut dkg, 3).await?;
                check_epoch_valid(&task, group_info_fetcher.clone())?;

                let justifications = dkg.get_justifications().await?;
                let justifications = parse_bundle(&justifications)?;

                phase3.run(&mut dkg, &justifications).await
            }
        };

        check_epoch_valid(&task, group_info_fetcher.clone())?;

        match result {
            Ok(output) => {
                println!("Success. Your share and threshold pubkey are ready.");

                // TODO Why isn't it working?
                // write_output(std::io::stdout(), &output)?;

                println!("{:#?}", output);

                println!("public key: {}", output.public.public_key());

                Ok(output)
            }
            Err(err) => Err(err.into()),
        }
    }
}

fn check_epoch_valid(
    task: &DKGTask,
    group_info_fetcher: Arc<RwLock<impl GroupInfoFetcher + Send + Sync>>,
) -> NodeResult<()> {
    let cache_index = group_info_fetcher.read().get_index()?;

    let cache_epoch = group_info_fetcher.read().get_epoch()?;

    if task.group_index != cache_index {
        return Err(NodeError::GroupIndexObsolete(cache_index));
    }

    if task.epoch < cache_epoch {
        return Err(NodeError::GroupEpochObsolete(cache_epoch));
    }

    Ok(())
}

async fn wait_for_phase(dkg: &mut impl CoordinatorViews, num: usize) -> NodeResult<()> {
    println!("Waiting for Phase {} to start", num);

    loop {
        let phase = dkg.in_phase().await?;

        if phase == num {
            break;
        }

        print!(".");

        // 1s for demonstration
        tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
    }

    println!("\nIn Phase {}. Moving to the next step.", num);

    Ok(())
}

fn parse_bundle<D: serde::de::DeserializeOwned>(bundle: &[Vec<u8>]) -> NodeResult<Vec<D>> {
    bundle
        .iter()
        .filter(|item| !item.is_empty()) // filter out empty items
        .map(|item| Ok(bincode::deserialize::<D>(item)?))
        .collect()
}

fn _write_output<W: Write>(writer: W, out: &DKGOutput<Curve>) -> NodeResult<()> {
    let output = OutputJson {
        public_key: hex::encode(&bincode::serialize(&out.public.public_key())?),
        public_polynomial: hex::encode(&bincode::serialize(&out.public)?),
        share: hex::encode(&bincode::serialize(&out.share)?),
    };
    serde_json::to_writer(writer, &output)?;
    Ok(())
}

#[derive(serde::Serialize, Debug)]
struct OutputJson {
    #[serde(rename = "publicKey")]
    public_key: String,
    #[serde(rename = "publicPolynomial")]
    public_polynomial: String,
    #[serde(rename = "share")]
    share: String,
}
