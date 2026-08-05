use crate::domain::{
    bip39::EntropyTarget,
    protocol::{CanonicalInputKind, ConversionProtocol, ProtocolSpecification, RejectionPolicy},
};

use super::{ContentBlock as B, Document, ProtocolMenuChoice};

#[must_use]
pub fn protocol_explanation(protocol: ConversionProtocol, target: EntropyTarget) -> Document {
    let specification = protocol.specification(target);
    match protocol {
        ConversionProtocol::ExactV1 => exact_explanation(specification, target),
        ConversionProtocol::WordExactV1 => {
            let RejectionPolicy::Localized {
                word_candidate_rolls,
                word_candidate_limit,
                tail_candidate_rolls,
                tail_candidate_limit,
            } = specification.rejection() else { unreachable!() };
            Document::new(
                "WORD-BY-WORD EXACT".to_owned(),
                vec![
                    B::Heading("WORD-BY-WORD EXACT · LOCALIZED REJECTION".to_owned()),
                    B::Heading("PURPOSE".to_owned()),
                    B::Paragraph("Build uniform BIP-39 entropy one 11-bit word position at a time. A rejected candidate discards only its small group; accepted positions remain kept.".to_owned()),
                    B::Heading("FULL WORD POSITIONS".to_owned()),
                    B::Paragraph(format!("{word_candidate_rolls} ordered D6 rolls have 46,656 outcomes. Interpret face minus one as big-endian base-6.")),
                    B::Verbatim({
                        let limit = grouped_decimal(word_candidate_limit);
                        format!("limit = {limit}\nx < {limit} → accept x mod 2,048\nx ≥ {limit} → reroll only these six dice")
                    }),
                    B::Paragraph("Each index has the same number of accepted preimages; hashing creates no entropy here.".to_owned()),
                    B::Heading("FINAL ENTROPY TAIL".to_owned()),
                    B::Paragraph(format!("The selected target uses {tail_candidate_rolls}-roll tail candidates with acceptance limit {tail_candidate_limit}. BIP-39 checksum bits are calculated afterward.")),
                ],
            )
        }
        ConversionProtocol::NativeHashV1 => Document::new(
            "NATIVE HASH".to_owned(),
            vec![
                B::Heading("NATIVE HASH · DETERMINISTIC CONDITIONING".to_owned()),
                B::Heading("PURPOSE".to_owned()),
                B::Paragraph("Condition every ordered roll through SHA-256 while binding protocol version, entropy target, and roll count.".to_owned()),
                B::Heading("CANONICAL INPUT".to_owned()),
                B::Paragraph(format!("Input kind: {:?}. The versioned binary header prevents the same face digits under another target or count from meaning the same input.", specification.canonical_input())),
                B::Paragraph("Hashing does not create entropy, certify fair dice, or repair a compromised device.".to_owned()),
                B::Paragraph(format!("The selected target requires exactly {} recorded rolls.", specification.minimum_observations())),
            ],
        ),
        ConversionProtocol::ColdcardV1 => coldcard_explanation(specification, target),
        ConversionProtocol::KeystoneLegacyV1 => {
            keystone_legacy_explanation(specification, target)
        }
        ConversionProtocol::JadeDirectV1 => Document::new(
            "JADE DIRECT WORDS".to_owned(),
            vec![
                B::Heading("JADE DIRECT WORDS · MIXED PHYSICAL DICE".to_owned()),
                B::Heading("PURPOSE".to_owned()),
                B::Paragraph("Use Blockstream Jade's published D16/D16/D8 table order for each complete entropy word position, then physically select the remaining entropy bits before calculating the checksum.".to_owned()),
                B::Heading("DIRECT WORD POSITIONS".to_owned()),
                B::Verbatim("index = 128 × (D16₁ - 1)\n      +   8 × (D16₂ - 1)\n      +       (D8 - 1)".to_owned()),
                B::Paragraph(format!("Repeat for the first {} positions. The published example 10, 9, 8 selects index 1,223: ocean.", target.word_count() - 1)),
                B::Heading("FINAL ENTROPY TAIL".to_owned()),
                B::Paragraph(if target == EntropyTarget::Words12 {
                    "Roll one D16 and one D8 to select the remaining seven entropy bits uniformly. BIP-39 then calculates four checksum bits and the final word.".to_owned()
                } else {
                    "Roll one D8 to select the remaining three entropy bits uniformly. BIP-39 then calculates eight checksum bits and the final word.".to_owned()
                }),
                B::Paragraph("Jade uses device-assisted random selection among valid final words. This ceremony replaces that random choice with explicit physical dice; it matches Jade's table and valid final-word set, not its complete interaction.".to_owned()),
                B::Paragraph(format!("The selected target requires exactly {} typed mixed-die observations. No hashing or rejection occurs.", specification.minimum_observations())),
            ],
        ),
        ConversionProtocol::BitBox02DirectV1 => bitbox_explanation(specification, target),
        ConversionProtocol::KruxD20V1 => krux_d20_explanation(target),
        ConversionProtocol::CoinFourD6DirectV1 => coin_four_d6_explanation(specification),
        ConversionProtocol::SeedSignerCoinsV1 => Document::new(
            "SEEDSIGNER COIN FLIPS".to_owned(),
            vec![
                B::Heading("SEEDSIGNER COIN FLIPS · ASCII BINARY HASH".to_owned()),
                B::Heading("PURPOSE".to_owned()),
                B::Paragraph("Reproduce SeedSigner's coin-flip conversion for the same ordered binary string and mnemonic length.".to_owned()),
                B::Heading("CANONICAL INPUT".to_owned()),
                B::Paragraph(format!("Record exactly {} physical flips as `0` for tails and `1` for heads. Hash those ASCII bytes with SHA-256—no separator or newline.", specification.minimum_observations())),
                B::Heading("OUTPUT".to_owned()),
                B::Paragraph(if target == EntropyTarget::Words12 {
                    "Use the first 16 digest bytes as BIP-39 entropy; BIP-39 calculates the four checksum bits.".to_owned()
                } else {
                    "Use all 32 digest bytes as BIP-39 entropy; BIP-39 calculates the eight checksum bits.".to_owned()
                }),
                B::Paragraph("Hashing conditions the recorded flips but does not create entropy or prove that the coin was fair. Changing one flip or its position changes the digest.".to_owned()),
            ],
        ),
    }
}

fn coin_four_d6_explanation(specification: ProtocolSpecification) -> Document {
    Document::new(
        "COIN + FOUR-D6 DIRECT WORDS".to_owned(),
        vec![
            B::Heading("COIN + FOUR-D6 · WHOLE-CANDIDATE REJECTION".to_owned()),
            B::Heading("PURPOSE".to_owned()),
            B::Paragraph("Reproduce the pinned Bip39-diceware direct-word table while calculating checksum locally instead of testing secret words in a wallet.".to_owned()),
            B::Heading("EACH TABLE CANDIDATE".to_owned()),
            B::Paragraph("Flip one coin, then roll four D6 in a fixed order. Heads covers indices 0–1,295. Tails covers 1,296–2,047 through tuple 4,3,6,2.".to_owned()),
            B::Verbatim("r = 216(d₁−1) + 36(d₂−1) + 6(d₃−1) + (d₄−1)\nheads: index = r\ntails and r < 752: index = 1296 + r\notherwise: reject coin + all four rolls".to_owned()),
            B::Paragraph("After rejection, use a fresh coin flip and four fresh rolls. Keeping any part of a rejected candidate breaks the table's uniform mapping.".to_owned()),
            B::Heading("ENTROPY AND CHECKSUM".to_owned()),
            B::Paragraph("Collect 12 accepted indices. Keep all 11 bits from the first 11 and the upper seven bits from the twelfth, forming 128 entropy bits. Calculate the four BIP-39 checksum bits and replace the twelfth candidate's low nibble.".to_owned()),
            B::Paragraph(format!("The rejection-free minimum is {} outcomes; each rejection adds five auditable outcomes. This source defines 12 words only.", specification.minimum_observations())),
            B::Paragraph("Compatibility covers the pinned table and corrected result, not wallet-assisted checksum searching. Rejection does not establish coin or dice fairness.".to_owned()),
        ],
    )
}

fn krux_d20_explanation(target: EntropyTarget) -> Document {
    Document::new(
        "KRUX D20".to_owned(),
        vec![
            B::Heading("KRUX D20 · HYPHENATED DECIMAL HASH".to_owned()),
            B::Heading("PURPOSE".to_owned()),
            B::Paragraph("Reproduce Krux's D20 roll-to-mnemonic conversion for the same ordered faces, final count, and target.".to_owned()),
            B::Heading("CANONICAL INPUT".to_owned()),
            B::Paragraph("Write each D20 face as unpadded decimal ASCII and place one ASCII hyphen between faces. Include no leading/trailing hyphen or newline.".to_owned()),
            B::Verbatim("faces:  1, 10, 20\nbytes:  31 2d 31 30 2d 32 30\ntext:   1-10-20".to_owned()),
            B::Heading("HASH AND OUTPUT".to_owned()),
            B::Paragraph(if target == EntropyTarget::Words12 {
                "After at least 30 rolls, hash the complete text once with SHA-256 and use the first 16 digest bytes as BIP-39 entropy.".to_owned()
            } else {
                "After at least 60 rolls, hash the complete text once with SHA-256 and use all 32 digest bytes as BIP-39 entropy.".to_owned()
            }),
            B::Paragraph("Additional D20 rolls are accepted and change the final digest. Hashing does not create entropy or establish die fairness.".to_owned()),
            B::Paragraph("Compatibility covers Krux's deterministic conversion. Its advisory Shannon estimate, pattern warning, keypad, and complete device interaction are outside this profile.".to_owned()),
        ],
    )
}

fn bitbox_explanation(specification: ProtocolSpecification, target: EntropyTarget) -> Document {
    Document::new(
        "BITBOX02 DIRECT WORDS".to_owned(),
        vec![
            B::Heading("BITBOX02 DIRECT WORDS · D6 REJECTION + COIN".to_owned()),
            B::Heading("PURPOSE".to_owned()),
            B::Paragraph("Reproduce BitBox02's published Diceware lookup order for complete entropy-word positions, then physically select the remaining entropy bits before calculating the checksum.".to_owned()),
            B::Heading("DIRECT WORD POSITIONS".to_owned()),
            B::Paragraph("For each position, accept five D6 faces in 1–4. Record and locally retry every 5 or 6. Then flip one coin.".to_owned()),
            B::Verbatim("digits = D6₁-1 … D6₅-1 (base 4, MSB first)\nselector = heads:0 · tails:1\nindex = 2 × base4(digits) + selector".to_owned()),
            B::Paragraph(format!("Repeat for the first {} positions. The table boundaries are all 1 + heads → abandon and all 4 + tails → zoo.", target.word_count() - 1)),
            B::Heading("FINAL ENTROPY TAIL".to_owned()),
            B::Paragraph(if target == EntropyTarget::Words12 {
                "Flip seven additional coins for the remaining seven entropy bits. Heads maps to 0 and tails to 1; BIP-39 then calculates four checksum bits and the final word.".to_owned()
            } else {
                "Flip three additional coins for the remaining three entropy bits. Heads maps to 0 and tails to 1; BIP-39 then calculates eight checksum bits and the final word.".to_owned()
            }),
            B::Paragraph("Capture keys remain 0 = tails and 1 = heads, so the displayed physical-side key is deliberately opposite the lookup selector bit. Follow the side labels, not an assumed bit value.".to_owned()),
            B::Paragraph("BitBox02 firmware does not capture these dice rolls. Its published workflow uses an external printed table, then asks the operator to enter the resulting words and choose from valid final words; 12-word entry uses autocomplete. This ceremony executes the printed-table procedure and replaces final-word selection with explicit tail coins. Compatibility is limited to the published table and valid final-word set.".to_owned()),
            B::Paragraph(format!("The minimum is {} outcomes plus any rejected D6 retries. Rejection does not repair biased dice or coins.", specification.minimum_observations())),
        ],
    )
}

#[must_use]
pub fn protocol_menu_explanation(choice: ProtocolMenuChoice, target: EntropyTarget) -> Document {
    if let Some(protocol) = choice.implemented_protocol(target) {
        return protocol_explanation(protocol, target);
    }

    let (purpose, procedure, boundary) = match choice {
        ProtocolMenuChoice::KeystoneLegacyDice => (
            "The legacy Keystone profile is available only for 24-word output.",
            "The published source establishes a 24-word workflow but does not establish how a legacy Keystone device produced 12 words from dice.",
            "Choose 24 words to use the implemented, vector-tested profile. A guessed 12-word adaptation would not be a compatibility protocol.",
        ),
        ProtocolMenuChoice::CoinFourD6Direct => (
            "The pinned coin-plus-four-D6 table defines only a 12-word procedure.",
            "Twelve accepted table candidates supply 128 entropy bits before checksum replacement; the source does not specify the 24-word entropy-tail shape.",
            "Choose 12 words to use the implemented profile. A 24-word extension would be a new protocol rather than sourced compatibility.",
        ),
        ProtocolMenuChoice::JadeDirectWords
        | ProtocolMenuChoice::BitBoxDiceware
        | ProtocolMenuChoice::KruxD20
        | ProtocolMenuChoice::SeedSignerCoins
        | ProtocolMenuChoice::Coldcard
        | ProtocolMenuChoice::WordExact
        | ProtocolMenuChoice::Exact => unreachable!(),
    };

    Document::new(
        format!("{} · UNAVAILABLE", choice.name().to_uppercase()),
        vec![
            B::Heading(format!(
                "{} · UNSUPPORTED TARGET",
                choice.name().to_uppercase()
            )),
            B::Heading("PURPOSE".to_owned()),
            B::Paragraph(purpose.to_owned()),
            B::Heading("DOCUMENTED SHAPE".to_owned()),
            B::Paragraph(procedure.to_owned()),
            B::Heading("IMPLEMENTATION BOUNDARY".to_owned()),
            B::Paragraph(boundary.to_owned()),
            B::Paragraph("This catalog entry cannot be selected for a ceremony and produces no entropy or mnemonic.".to_owned()),
        ],
    )
}

fn exact_explanation(specification: ProtocolSpecification, target: EntropyTarget) -> Document {
    let (entropy, checksum_hash, checksum_bits, indices, words) = match target {
        EntropyTarget::Words12 => (
            "00000000000000000000000000000000",
            "374708fff7719dd5979ec875d56cd228\n6f6d3cf7ec317a3b25632aab28ec37bb",
            "0011",
            "0 0 0 0 0 0 0 0 0 0 0 3",
            "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about",
        ),
        EntropyTarget::Words24 => (
            "00000000000000000000000000000000\n00000000000000000000000000000000",
            "66687aadf862bd776c8fc18b8e9f8e20\n089714856ee233b3902a591d0d5f2925",
            "01100110",
            "0 0 0 0 0 0 0 0 0 0 0 0\n0 0 0 0 0 0 0 0 0 0 0 102",
            "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon art",
        ),
    };
    Document::new(
        "EXACT REJECTION".to_owned(),
        vec![
            B::Heading("SMALL EXAMPLE · TWO DICE TO EIGHT VALUES".to_owned()),
            B::Paragraph("Suppose we roll two six-sided dice and want one of 8 equally likely values, numbered 0–7.".to_owned()),
            B::Paragraph("Two ordered dice have 36 possible outcomes, written as (first roll, second roll):".to_owned()),
            B::Verbatim("(1, 1) → 0\n(1, 2) → 1\n…\n(6, 6) → 35".to_owned()),
            B::Paragraph("But 36 does not divide evenly by 8:".to_owned()),
            B::Verbatim("36 = 4 × 8 + 4".to_owned()),
            B::Paragraph("Using number mod 8 for every outcome would make values 0–3 occur five times and values 4–7 only four times. That is biased.".to_owned()),
            B::Paragraph("Exact instead accepts only the first 32 outcomes, then maps accepted values modulo 8.".to_owned()),
            B::Verbatim("0–31: accepted → number mod 8\n32–35: rejected".to_owned()),
            B::Paragraph("Concretely:".to_owned()),
            B::Verbatim("(1, 1) through (6, 2) → accepted\n(6, 3), (6, 4), (6, 5), (6, 6) → rejected".to_owned()),
            B::Paragraph(format!("The real {} protocol applies the same whole-sequence rejection rule to exactly {} rolls.", specification.id(), specification.minimum_observations())),
            B::Heading("WHEN OUTPUT EXISTS".to_owned()),
            B::Paragraph("Partial roll sequences do not produce candidate target entropy or a mnemonic. After the exact count is recorded, the complete sequence either maps to the full target or is rejected. A rejection requires a fresh complete attempt; appending another roll is not allowed.".to_owned()),
            B::Paragraph("The roll count establishes raw outcome capacity only under the fair, independent-D6 assumption; it does not measure the physical dice's actual entropy.".to_owned()),
            B::Heading("PUBLIC ACCEPTED EXAMPLE · NEVER USE AS A SEED".to_owned()),
            B::Heading("1 · MAP THE COMPLETE ROLL SEQUENCE".to_owned()),
            B::Paragraph(format!("Use exactly {} ordered rolls. Map each face to face minus one and read the resulting digits as one big-endian base-6 integer x.", specification.minimum_observations())),
            B::Verbatim(format!("rolls:  {} × face 1\ndigits: {} × base-6 digit 0\nx = 0", specification.minimum_observations(), specification.minimum_observations())),
            B::Heading("2 · APPLY REJECTION, THEN TAKE THE ENTROPY".to_owned()),
            B::Paragraph(format!("Compute limit = floor(6^n / 2^{0}) × 2^{0}. Accept only x < limit; then encode x mod 2^{0} as the target-width big-endian entropy.", target.entropy_bits())),
            B::Verbatim(format!("x = 0 → accepted\nentropy =\n{entropy}")),
            B::Paragraph("No SHA-256 is used to convert the rolls. Rejection is what gives every target value the same number of accepted roll sequences under the fair-dice model.".to_owned()),
            B::Heading("3 · CALCULATE THE BIP-39 CHECKSUM".to_owned()),
            B::Paragraph(format!("Hash the accepted entropy, then take its first {} bits. These calculated bits add error detection, not entropy.", target.checksum_bits())),
            B::Verbatim(format!("SHA-256(entropy) =\n{checksum_hash}\nchecksum bits = {checksum_bits}")),
            B::Heading("4 · SPLIT INTO 11-BIT WORD INDICES".to_owned()),
            B::Paragraph("Append the checksum bits to the entropy bits, split from left to right into 11-bit values, and use each value as a zero-based BIP-39 English-list index.".to_owned()),
            B::Verbatim(format!("indices =\n{indices}")),
            B::Heading(format!("PUBLIC {}-WORD RESULT", target.word_count())),
            B::Paragraph(words.to_owned()),
            B::Heading("SAME FORMAT, DIFFERENT RESULT".to_owned()),
            B::Paragraph("An accepted sequence supplies 128 or 256 entropy bits to the same standard BIP-39 checksum and word steps shown by COLDCARD. COLDCARD reaches that format by hashing the rolls, so the same rolls will normally produce different entropy and words.".to_owned()),
        ],
    )
}

fn coldcard_explanation(specification: ProtocolSpecification, target: EntropyTarget) -> Document {
    let CanonicalInputKind::AsciiFaceDigits = specification.canonical_input() else {
        unreachable!()
    };
    let (entropy, checksum_hash, checksum_bits, indices, words) = match target {
        EntropyTarget::Words12 => (
            "8d969eef6ecad3c29a3a629280e686cf",
            "1c965835c80f76dcce3662f81eb2375b\n2e028a67d5c0d998d3a58e94451c5720",
            "0001",
            "1132 1447 1502 1772 1385 1802\n839 610 1172 57 1293 1265",
            "mirror reject rookie talk pudding throw happy era myth already payment owner",
        ),
        EntropyTarget::Words24 => (
            "8d969eef6ecad3c29a3a629280e686cf\n0c3f5d5a86aff3ca12020c923adc6c92",
            "ff7f73b854845fc02aa13b777ac090fb\n1d9ebfe16c8950c7d26499371dd0b479",
            "11111111",
            "1132 1447 1502 1772 1385 1802\n839 610 1172 57 1293 1264\n1567 1397 848 1711 1950 644\n1028 201 285 881 1426 767",
            "mirror reject rookie talk pudding throw happy era myth already payment own sentence push head sting video explain letter bomb casual hotel rather garment",
        ),
    };
    Document::new(
        "COLDCARD HASH".to_owned(),
        vec![
            B::Heading("COLDCARD HASH · DEVICE COMPATIBILITY".to_owned()),
            B::Heading("COMPATIBILITY BOUNDARY".to_owned()),
            B::Paragraph("SeedSigner and Krux D6 document the same core conversion: SHA-256 over the completed ASCII face-digit string, then the first 128 bits for 12 words or all 256 bits for 24. SeedSigner fixes capture at 50 or 99 rolls; Krux and COLDCARD may permit different finalization choices. Matching a completed string does not guarantee an identical ceremony.".to_owned()),
            B::Paragraph("Do not expect the same rolls to match other dice workflows. Legacy Keystone maps face 6 to character 0 before hashing. Jade selects words directly with two D16s and one D8. BitBox02 uses D6 rejection, base-4 digits, and a coin flip. Krux D20 is also a distinct protocol.".to_owned()),
            B::Paragraph("A wallet that calculates the final BIP-39 word from 11 or 23 supplied words is only providing a checksum helper. That feature, available in wallets including Jade and Passport, does not define or reproduce this roll-to-mnemonic conversion.".to_owned()),
            B::Heading("PUBLIC SHORT EXAMPLE · NEVER USE AS A SEED".to_owned()),
            B::Paragraph("COLDCARD documents the six-roll example 1, 2, 3, 4, 5, 6. Following its byte-for-byte procedure reproduces COLDCARD output. This example has only about 15 bits of entropy and must never be used as a seed.".to_owned()),
            B::Heading("1 · START WITH THE EMPTY ROLL STRING".to_owned()),
            B::Paragraph("Before any roll, COLDCARD initializes SHA-256 with an empty input and displays that candidate digest.".to_owned()),
            B::Verbatim("rolls:  (none)\ntext:   \nSHA-256(empty) =\ne3b0c44298fc1c149afbf4c8996fb924\n27ae41e4649b934ca495991b7852b855".to_owned()),
            B::Heading("2 · APPEND AND HASH EACH RECORDED ROLL".to_owned()),
            B::Paragraph("For each roll, append its one-byte ASCII digit to the accumulated text—without spaces, commas, a prefix, or a newline—then display the digest for all rolls recorded so far.".to_owned()),
            B::Verbatim("after 1:       SHA-256(1)\nafter 1, 2:    SHA-256(12)\n…\nafter 1…6:     SHA-256(123456) =\n8d969eef6ecad3c29a3a629280e686cf\n0c3f5d5a86aff3ca12020c923adc6c92\n\nfinal text:    123456\nfinal bytes:   31 32 33 34 35 36  (hex)".to_owned()),
            B::Paragraph("This is one running SHA-256 input stream, not separate chunk hashes: each displayed digest equals SHA-256 of the complete accumulated string. The preview digest is not fed back into the next update.".to_owned()),
            B::Paragraph("Each new roll changes the whole candidate digest; it does not preserve a growing prefix of the eventual result. COLDCARD displays candidates from the empty input onward, but Dice Rolls Only does not finalize the mnemonic below the required minimum.".to_owned()),
            B::Paragraph("SHA-256 conditions the input; it does not create entropy or establish that the die was fair. More fair, independent rolls increase source outcome capacity, but the digest's 256-bit size never proves 256 bits of source entropy.".to_owned()),
            B::Paragraph("At finalization, COLDCARD rejects the whole attempt when any face occurred more than 30% of the time. Equality at 30% is accepted. This heuristic does not prove that an accepted die was fair.".to_owned()),
            B::Heading(format!("3 · TAKE {} BITS AS ENTROPY", target.entropy_bits())),
            B::Paragraph(if target == EntropyTarget::Words12 {
                "For 12 words, keep the first 16 digest bytes and discard the remaining 16.".to_owned()
            } else {
                "For 24 words, keep all 32 digest bytes.".to_owned()
            }),
            B::Verbatim(format!("entropy =\n{entropy}")),
            B::Heading("4 · CALCULATE THE BIP-39 CHECKSUM".to_owned()),
            B::Paragraph(format!("Hash the entropy, then take its first {} bits. These calculated bits add error detection, not entropy.", target.checksum_bits())),
            B::Verbatim(format!("SHA-256(entropy) =\n{checksum_hash}\nchecksum bits = {checksum_bits}")),
            B::Heading("5 · SPLIT INTO 11-BIT WORD INDICES".to_owned()),
            B::Paragraph("Append the checksum bits to the entropy bits, split from left to right into 11-bit values, and use each value as a zero-based BIP-39 English-list index.".to_owned()),
            B::Verbatim(format!("indices =\n{indices}")),
            B::Heading(format!("PUBLIC {}-WORD RESULT", target.word_count())),
            B::Paragraph(words.to_owned()),
            B::Heading("MINIMUM, THEN OPTIONAL EXTRA ROLLS".to_owned()),
            B::Paragraph(format!("COLDCARD's Dice Rolls Only workflow enforces at least {} rolls for this target before finalization; additional rolls are allowed. A sequence that passes the distribution guard deterministically produces one result.", specification.minimum_observations())),
            B::Paragraph("This reaches the same 128- or 256-bit BIP-39 input format as Exact, not the same value. Exact uses fixed-count rejection instead of hashing, so the same rolls will normally produce different entropy and words.".to_owned()),
        ],
    )
}

fn keystone_legacy_explanation(
    specification: ProtocolSpecification,
    target: EntropyTarget,
) -> Document {
    assert_eq!(target, EntropyTarget::Words24);
    Document::new(
        "KEYSTONE LEGACY HASH".to_owned(),
        vec![
            B::Heading("KEYSTONE LEGACY HASH · 24 WORDS ONLY".to_owned()),
            B::Heading("COMPATIBILITY SCOPE".to_owned()),
            B::Paragraph("Reproduce the legacy Keystone/Ian Coleman dice conversion documented by Keystone. This profile does not claim compatibility with Keystone 3 or with undocumented firmware versions.".to_owned()),
            B::Heading("1 · MAP EACH D6 FACE TO ONE ASCII BYTE".to_owned()),
            B::Paragraph("Append faces without spaces, separators, a prefix, or a newline. Faces 1 through 5 keep their digit; face 6 becomes character 0.".to_owned()),
            B::Verbatim("rolls:         1 2 3 4 5 6\nmapped text:   123450\nmapped bytes:  31 32 33 34 35 30  (hex)".to_owned()),
            B::Paragraph("The 6→0 mapping is why this does not match COLDCARD for a sequence containing face 6.".to_owned()),
            B::Heading("2 · HASH THE COMPLETE MAPPED STRING".to_owned()),
            B::Paragraph("Apply SHA-256 once to the complete mapped ASCII string. Do not hash each roll separately and do not feed an intermediate digest back into the next update.".to_owned()),
            B::Heading("3 · USE THE FULL DIGEST AS BIP-39 ENTROPY".to_owned()),
            B::Paragraph("Use all 32 digest bytes as 256-bit entropy, calculate the eight-bit BIP-39 checksum, and split the resulting 264 bits into 24 English word indices.".to_owned()),
            B::Heading("ROLL REQUIREMENT".to_owned()),
            B::Paragraph(format!("Legacy Keystone permits generation from {} D6 rolls, warns that the entropy may be insufficient, and recommends continuing to 99. This profile matches that completion threshold and accepts additional rolls. A 24-word mnemonic does not make 50 physical rolls a 256-bit entropy source.", specification.minimum_observations())),
            B::Paragraph("Same mapped rolls and target produce the same mnemonic. Hashing does not create entropy or establish that the physical die was fair.".to_owned()),
        ],
    )
}

fn grouped_decimal(value: u32) -> String {
    let digits = value.to_string();
    let first = digits.len() % 3;
    let mut output = String::new();
    if first > 0 {
        output.push_str(&digits[..first]);
    }
    for chunk in digits.as_bytes()[first..].chunks(3) {
        if !output.is_empty() {
            output.push(',');
        }
        output.push_str(core::str::from_utf8(chunk).expect("decimal digits are UTF-8"));
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unsupported_target_documents_are_explicitly_non_operational() {
        for (choice, target) in [
            (
                ProtocolMenuChoice::KeystoneLegacyDice,
                EntropyTarget::Words12,
            ),
            (ProtocolMenuChoice::CoinFourD6Direct, EntropyTarget::Words24),
        ] {
            let text = format!("{:?}", protocol_menu_explanation(choice, target).blocks());
            assert!(text.contains("UNSUPPORTED TARGET"));
            assert!(text.contains("cannot be selected"));
        }
        let coins = format!(
            "{:?}",
            protocol_menu_explanation(ProtocolMenuChoice::SeedSignerCoins, EntropyTarget::Words12,)
                .blocks()
        );
        assert!(coins.contains("exactly 128 physical flips"));
        assert!(coins.contains("ASCII bytes with SHA-256"));
    }

    #[test]
    fn keystone_document_is_target_limited_and_exact_about_mapping() {
        let text = format!(
            "{:?}",
            protocol_menu_explanation(
                ProtocolMenuChoice::KeystoneLegacyDice,
                EntropyTarget::Words24,
            )
            .blocks()
        );
        assert!(text.contains("24 WORDS ONLY"));
        assert!(text.contains("mapped text:   123450"));
        assert!(text.contains("permits generation from 50 D6 rolls"));
        assert!(text.contains("recommends continuing to 99"));

        let unsupported = format!(
            "{:?}",
            protocol_menu_explanation(
                ProtocolMenuChoice::KeystoneLegacyDice,
                EntropyTarget::Words12,
            )
            .blocks()
        );
        assert!(unsupported.contains("UNSUPPORTED TARGET"));
        assert!(unsupported.contains("available only for 24-word output"));
    }

    #[test]
    fn jade_document_scopes_table_compatibility_and_physical_tail() {
        let text = format!(
            "{:?}",
            protocol_explanation(ConversionProtocol::JadeDirectV1, EntropyTarget::Words12).blocks()
        );
        assert!(text.contains("10, 9, 8 selects index 1,223: ocean"));
        assert!(text.contains("remaining seven entropy bits"));
        assert!(text.contains("replaces that random choice with explicit physical dice"));
        assert!(text.contains("exactly 35 typed mixed-die observations"));
    }

    #[test]
    fn bitbox_document_fixes_table_order_and_scoped_tail_extension() {
        let text = format!(
            "{:?}",
            protocol_explanation(ConversionProtocol::BitBox02DirectV1, EntropyTarget::Words24,)
                .blocks()
        );
        assert!(text.contains("heads:0 · tails:1"));
        assert!(text.contains("all 1 + heads → abandon"));
        assert!(text.contains("all 4 + tails → zoo"));
        assert!(text.contains("Flip three additional coins"));
        assert!(text.contains("firmware does not capture these dice rolls"));
        assert!(text.contains("Compatibility is limited to the published table"));
    }

    #[test]
    fn krux_document_fixes_decimal_separator_and_compatibility_scope() {
        let text = format!(
            "{:?}",
            protocol_explanation(ConversionProtocol::KruxD20V1, EntropyTarget::Words12).blocks()
        );
        assert!(text.contains("31 2d 31 30 2d 32 30"));
        assert!(text.contains("at least 30 rolls"));
        assert!(text.contains("Additional D20 rolls are accepted"));
        assert!(text.contains("Shannon estimate"));
    }

    #[test]
    fn coin_four_d6_document_fixes_rejection_and_checksum_scope() {
        let text = format!(
            "{:?}",
            protocol_explanation(
                ConversionProtocol::CoinFourD6DirectV1,
                EntropyTarget::Words12,
            )
            .blocks()
        );
        assert!(text.contains("tails and r < 752"));
        assert!(text.contains("reject coin + all four rolls"));
        assert!(text.contains("upper seven bits"));
        assert!(text.contains("replace the twelfth candidate's low nibble"));
        assert!(text.contains("12 words only"));
    }

    #[test]
    fn word_exact_document_uses_domain_candidate_parameters() {
        let document =
            protocol_explanation(ConversionProtocol::WordExactV1, EntropyTarget::Words12);
        let text = format!("{:?}", document.blocks());
        assert!(text.contains("45,056"));
        assert!(text.contains("4-roll tail"));
        assert!(text.contains("reroll only these six dice"));
    }

    #[test]
    fn coldcard_document_traces_the_public_example_for_12_words() {
        let text = format!(
            "{:?}",
            protocol_explanation(ConversionProtocol::ColdcardV1, EntropyTarget::Words12).blocks()
        );
        for expected in [
            "SHA-256(empty)",
            "SHA-256(12)",
            "31 32 33 34 35 36",
            "8d969eef6ecad3c29a3a629280e686cf",
            "checksum bits = 0001",
            "payment owner",
            "enforces at least 50 rolls",
            "more than 30% of the time",
            "not separate chunk hashes",
            "changes the whole candidate digest",
            "SeedSigner and Krux D6",
            "Legacy Keystone maps face 6 to character 0",
            "Jade and Passport",
        ] {
            assert!(text.contains(expected));
        }
    }

    #[test]
    fn coldcard_document_traces_the_public_example_for_24_words() {
        let text = format!(
            "{:?}",
            protocol_explanation(ConversionProtocol::ColdcardV1, EntropyTarget::Words24).blocks()
        );
        for expected in [
            "0c3f5d5a86aff3ca12020c923adc6c92",
            "checksum bits = 11111111",
            "rather garment",
            "enforces at least 99 rolls",
            "same 128- or 256-bit BIP-39 input format",
        ] {
            assert!(text.contains(expected));
        }
    }

    #[test]
    fn exact_document_traces_an_accepted_12_word_example() {
        let text = format!(
            "{:?}",
            protocol_explanation(ConversionProtocol::ExactV1, EntropyTarget::Words12).blocks()
        );
        for expected in [
            "Partial roll sequences do not produce",
            "fresh complete attempt",
            "50 × face 1",
            "x = 0 → accepted",
            "374708fff7719dd5979ec875d56cd228",
            "checksum bits = 0011",
            "abandon abandon abandon",
            "same standard BIP-39 checksum",
        ] {
            assert!(text.contains(expected));
        }
    }

    #[test]
    fn exact_document_traces_an_accepted_24_word_example() {
        let text = format!(
            "{:?}",
            protocol_explanation(ConversionProtocol::ExactV1, EntropyTarget::Words24).blocks()
        );
        for expected in [
            "100 × face 1",
            "66687aadf862bd776c8fc18b8e9f8e20",
            "checksum bits = 01100110",
            "abandon art",
        ] {
            assert!(text.contains(expected));
        }
    }
}
