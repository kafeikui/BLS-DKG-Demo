use std::{fs::File, io::Write, path::PathBuf};

use ethers::{prelude::Abigen, utils::Solc};

const CONTRACT_PATH: &str = "../../solidity/contracts/DKG.sol";
const SOLC_PATH: &str = "../../solidity/bin/solc-linux-amd64-v0.6.6+commit.6c089d02";

// Generates the bindings under `src/`
fn main() {
    // Only re-run the builder script if the contract changes
    println!("cargo:rerun-if-changed={}", CONTRACT_PATH);

    let solc_path: PathBuf = PathBuf::from(SOLC_PATH);

    // compile the DKG contract (requires solc on the builder's system)
    let contracts = Solc::new(CONTRACT_PATH)
        .solc_path(solc_path)
        .build_raw()
        .expect("could not compile");
    let contract = contracts.get("DKG").expect("contract not found");

    let abi = contract.abi.clone();

    let mut f = File::create("dkg.bin").expect("could not create DKG bytecode file");
    f.write_all(contract.bin.as_bytes())
        .expect("could not write DKG bytecode to the file");

    // generate type-safe bindings to it
    let bindings = Abigen::new("DKG", abi)
        .expect("could not instantiate Abigen")
        .generate()
        .expect("could not generate bindings");
    bindings
        .write_to_file("./src/dkg_contract.rs")
        .expect("could not write bindings to file");
}
