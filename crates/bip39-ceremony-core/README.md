# bip39-ceremony-core

Deterministic, inspectable conversion of typed physical dice and coin observations
into English BIP-39 mnemonics.

The crate implements the versioned conversion protocols used by the
[`bip39-ceremony`](https://github.com/vindard/bip39-ceremony) TUI without importing
its ceremony, safety, presentation, or terminal policy.

> Roll sequences, entropy, calculation evidence, and mnemonic words are secret.
> Secret-bearing types redact `Debug` and zeroize owned buffers where practical,
> but Rust and this crate cannot guarantee complete erasure from an allocator,
> swap, crash dumps, or the host operating system.

```rust
use bip39_ceremony_core::{
    calculate, CalculationOutcome, Capture, ConversionProtocol, DieFace,
    EntropyTarget, RollSequence,
};

let mut rolls = RollSequence::new();
for _ in 0..50 {
    rolls.push(DieFace::new(1).expect("one is a valid D6 face"));
}

let outcome = calculate(
    EntropyTarget::Words12,
    ConversionProtocol::ExactV1,
    Capture::Dice(&rolls),
)?;

let CalculationOutcome::Accepted(calculation) = outcome else {
    return Ok(()); // whole-stream Exact may reject a valid-length capture
};
assert_eq!(calculation.mnemonic().words()[0], "abandon");
# Ok::<(), Box<dyn std::error::Error>>(())
```

Hashing conditions captured observations; it does not create entropy. Callers
remain responsible for the physical entropy source, secret handling, and safe
mnemonic use.
