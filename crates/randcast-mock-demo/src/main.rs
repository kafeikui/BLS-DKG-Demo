use dkg_core::primitives::{
    group::{Group, Node},
    joint_feldman,
    types::DKGOutput,
};
use dkg_core::{DKGPhase, Phase2Result};
use randcast_mock_demo::{
    contract::controller::*, node::errors::NodeResult, test_helpers::InMemoryBoard,
};
use std::collections::HashMap;
use threshold_bls::{
    curve::bls12381::{self, PairingCurve as BLS12_381},
    group::Curve,
    poly::{Eval, Idx},
    sig::{G1Scheme, Scheme, SignatureScheme, ThresholdScheme},
};

/// This a demo script demonstrating general randcast workflow.
#[tokio::main]
async fn main() -> NodeResult<()> {
    let initial_entropy = 0x8762_4875_6548_6346;

    println!(
        "controller is deploying... initial entropy: {}",
        initial_entropy
    );

    let mut controller = Controller::new(initial_entropy);

    let (t, n) = (3, 5);

    println!("nodes setup... t: {} n: {}", t, n);

    let rng = &mut rand::thread_rng();

    let (mut board, phase0s) = setup::<bls12381::Curve, G1Scheme<BLS12_381>, _>(n, t, rng);

    println!("nodes are registering to controller...");

    phase0s.iter().enumerate().for_each(|(i, phase0)| {
        controller
            .node_register(
                format!("0x{}", i),
                bincode::serialize(&phase0.info.public_key).unwrap(),
            )
            .unwrap();
    });

    println!("DKG task is emitting...");

    let dkg_task = controller.emit_dkg_task()?;

    let group_index = dkg_task.group_index;

    let group_epoch = dkg_task.epoch;

    // executes the DKG state machine and ensures that the keys are generated correctly
    let outputs = run_dkg::<bls12381::Curve, G1Scheme<BLS12_381>>(&mut board, phase0s).await;

    // get the public key (we have already checked that all outputs' pubkeys are the same)
    let public_poly = outputs[0].public.clone();

    let pubkey = public_poly.public_key();

    let pp1 = &outputs[1].public;
    let pp2 = &outputs[2].public;

    let ppp1 = outputs[1].public.eval(2).value;
    let ppp2 = outputs[2].public.eval(2).value;

    // note they're the same point although print outputs are different
    println!("{:?}", ppp1);
    println!("{:?}", ppp2);

    println!("DKG result is committing...");

    (0..n).for_each(|i| {
        let res = controller.commit_dkg(
            format!("0x{}", i),
            group_index,
            group_epoch,
            bincode::serialize(&pubkey).unwrap(),
            bincode::serialize(&public_poly.eval(i as u32).value).unwrap(),
            vec![],
        );
        println!("{}-res: {:?}", i, res);
    });

    let group = controller.get_group(1);

    println!("group state: {}", group.state);

    println!("group committers: {:?}", group.committers);

    // let msg = rand::random::<[u8; 32]>().to_vec();

    let msg = String::from("ujehwsndfgljkhrlkg");

    println!("An user is requesting a randomness... msg seed: {}", msg);

    let request_res = controller.request(&msg);

    println!("request_res: {:?}", request_res);

    println!("A signature task is emitting...");

    let signature_task = controller.emit_signature_task()?;

    let signature_index = signature_task.index;

    // generates a partial sig with each share from the dkg
    let partial_sigs = outputs
        .iter()
        .map(|output| G1Scheme::<BLS12_381>::partial_sign(&output.share, msg.as_bytes()).unwrap())
        .collect::<Vec<_>>();

    // committer verify the partial threshold signatures first
    partial_sigs
        .iter()
        .enumerate()
        .for_each(|(i, partial_sig)| {
            G1Scheme::<BLS12_381>::partial_verify(&public_poly, msg.as_bytes(), partial_sig)
                .unwrap();

            if i == 2 {
                G1Scheme::<BLS12_381>::partial_verify(pp1, msg.as_bytes(), partial_sig).unwrap();
                G1Scheme::<BLS12_381>::partial_verify(pp2, msg.as_bytes(), partial_sig).unwrap();

                let partial_2: Eval<Vec<u8>> = bincode::deserialize(partial_sig).unwrap();
                G1Scheme::<BLS12_381>::verify(&ppp1, msg.as_bytes(), &partial_2.value).unwrap();
                G1Scheme::<BLS12_381>::verify(&ppp2, msg.as_bytes(), &partial_2.value).unwrap();
            }
        });

    // then aggregates them
    let sig = G1Scheme::<BLS12_381>::aggregate(t, &partial_sigs).unwrap();

    // committer verify the threshold signature first
    G1Scheme::<BLS12_381>::verify(pubkey, msg.as_bytes(), &sig).unwrap();

    println!("Committers are committing result of the signature task...");

    (0..n).for_each(|i| {
        // the participant list to be rewarded by this signature task
        let mut partial_signatures: HashMap<String, Vec<u8>> = HashMap::new();

        partial_sigs
            .iter()
            .enumerate()
            .for_each(|(i, partial_sig)| {
                partial_signatures.insert(format!("0x{}", i), partial_sig.clone());
            });

        println!(
            "{}-res: {:?}",
            i,
            controller.fulfill(
                &format!("0x{}", i),
                1,
                signature_index,
                sig.clone(),
                partial_signatures,
            )
        )
    });

    let randomness_output = controller.get_last_output();

    println!("randomness output: {}", randomness_output);

    println!("finish.");

    Ok(())
}

async fn run_dkg<C, S>(
    board: &mut InMemoryBoard<C>,
    phase0s: Vec<joint_feldman::DKG<C>>,
) -> Vec<DKGOutput<C>>
where
    C: Curve,
    // We need to bind the Curve's Point and Scalars to the Scheme
    S: Scheme<Public = <C as Curve>::Point, Private = <C as Curve>::Scalar>,
{
    // Phase 1: Publishes shares
    let mut phase1s = Vec::new();
    for phase0 in phase0s {
        phase1s.push(phase0.run(board, rand::thread_rng).await.unwrap());
    }

    // Get the shares from the board
    let shares = board.shares.clone();

    // Phase2
    let mut phase2s = Vec::new();
    for phase1 in phase1s {
        phase2s.push(phase1.run(board, &shares).await.unwrap());
    }

    // Get the responses from the board
    let responses = board.responses.clone();

    let mut results = Vec::new();
    for phase2 in phase2s {
        results.push(phase2.run(board, &responses).await.unwrap());
    }

    // The distributed public key must be the same
    let outputs = results
        .into_iter()
        .map(|res| match res {
            Phase2Result::Output(out) => out,
            Phase2Result::GoToPhase3(_) => unreachable!("should not get here"),
        })
        .collect::<Vec<_>>();
    assert!(is_all_same(outputs.iter().map(|output| {
        // println!("{:?}", output.public);
        &output.public
    })));

    outputs
}

fn setup<C, S, R: rand::RngCore>(
    n: usize,
    t: usize,
    rng: &mut R,
) -> (InMemoryBoard<C>, Vec<joint_feldman::DKG<C>>)
where
    C: Curve,
    // We need to bind the Curve's Point and Scalars to the Scheme
    S: Scheme<Public = C::Point, Private = <C as Curve>::Scalar>,
{
    // generate a keypair per participant
    let keypairs = (0..n).map(|_| S::keypair(rng)).collect::<Vec<_>>();
    // keypairs
    //     .iter()
    //     .for_each(|(private, public)| println!("{} {}", private, public));

    let nodes = keypairs
        .iter()
        .enumerate()
        .map(|(i, (_, public))| {
            // println!("{}", i);
            Node::<C>::new(i as Idx, public.clone())
        })
        .collect::<Vec<_>>();

    // This is setup phase during which publickeys and indexes must be exchanged
    // across participants
    let group = Group::new(nodes, t).unwrap();

    // Create the Phase 0 for each participant
    let phase0s = keypairs
        .iter()
        .map(|(private, _)| joint_feldman::DKG::new(private.clone(), group.clone()).unwrap())
        .collect::<Vec<_>>();

    // Create the board
    let board = InMemoryBoard::<C>::new();

    (board, phase0s)
}

fn is_all_same<T: PartialEq>(mut arr: impl Iterator<Item = T>) -> bool {
    let first = arr.next().unwrap();
    arr.all(|item| item == first)
}
