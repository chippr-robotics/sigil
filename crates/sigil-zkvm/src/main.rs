//! SP1 zkVM signing program entry point
//!
//! This program runs inside the SP1 zkVM to produce provable ECDSA signatures
//! from MPC presignature shares.

#![no_main]
sp1_zkvm::entrypoint!(main);

use sigil_zkvm::{complete_presig, SigningInput, SigningOutput};

pub fn main() {
    // Read the signing input from the prover
    let input: SigningInput = sp1_zkvm::io::read();

    // Complete the signature
    let output = complete_presig(&input).expect("Signing failed");

    // Commit the output (public)
    sp1_zkvm::io::commit(&output);
}
