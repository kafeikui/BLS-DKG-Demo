fn main() -> Result<(), Box<dyn std::error::Error>> {
    let protos = &["proto/controller.proto", "proto/coordinator.proto"];

    for proto in protos {
        println!("cargo:rerun-if-changed={}", proto);
    }

    tonic_build::configure()
        .out_dir("stub")
        .compile(protos, &["proto"])?;

    Ok(())
}
