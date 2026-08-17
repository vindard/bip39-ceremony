//! Guidance on the one part of a dice ceremony this application cannot check:
//! how the dice were actually thrown.
//!
//! Every protocol here is deterministic given the recorded outcomes, so the
//! whole security of the result rests on those outcomes being unpredictable.
//! That is a physical property. The figures below are cited so an operator can
//! check them rather than take this screen's word for it.

use super::{ContentBlock as B, Document};

/// Where a die's own fairness stops mattering and the throw starts.
#[must_use]
pub fn physical_entropy_guidance() -> Document {
    Document::new(
        "PHYSICAL ENTROPY".to_owned(),
        vec![
            B::Heading("PHYSICAL ENTROPY · HOW YOU THROW".to_owned()),
            B::Heading("WHAT THIS APPLICATION CANNOT SEE".to_owned()),
            B::Paragraph("Every protocol here is deterministic: the same outcomes always give the same mnemonic. So all of the unpredictability has to come from the dice. This screen is the part of the ceremony no software can verify for you.".to_owned()),
            B::Heading("1 · THE DIE MATTERS LESS THAN YOU THINK".to_owned()),
            B::Paragraph("Cheap dice have measurable bias, and it costs almost nothing. Against the 255.9 bits that 99 fair rolls carry into a 24-word seed:".to_owned()),
            B::Verbatim("perfect die         255.9 bits\n2% bias             253.1 bits   (−2.9)\none face 8% high    244.9 bits   (−11.0)\none face +20%       229.9 bits   (−26.0)\none face +50%       198.0 bits   (−57.9)".to_owned()),
            B::Paragraph("The largest bias ever measured in ordinary dice is about 1.4%. Labby tallied 315,672 rolls of twelve cheap plastic dice and found the worst face 1.3% high; Iversen and colleagues threw 4,380,000 and put inexpensive drilled dice at roughly 1.4% per face.".to_owned()),
            B::Paragraph("Any bias big enough to reach the bottom of that table is big enough to notice casually. Buying casino dice does not buy a better seed.".to_owned()),
            B::Heading("2 · THE THROW IS WHAT MOVES".to_owned()),
            B::Paragraph("A die can be fair by symmetry and still be unfair by dynamics. Kapitaniak and colleagues measured how often a die ends up with the same face down as it started:".to_owned()),
            B::Verbatim("dropped, soft surface, no bounce    54.8%\nthrown with 4-5 bounces            19.9%\nperfectly fair                     16.7%".to_owned()),
            B::Paragraph("A die set down gently keeps its starting orientation more than half the time. That is a far larger effect than any manufacturing bias in the table above, and it is entirely under your control.".to_owned()),
            B::Heading("3 · WHAT TO ACTUALLY DO".to_owned()),
            B::Paragraph("Give every roll a thorough tumble. It is the one thing here that has been measured to matter.".to_owned()),
            B::Paragraph("A closed box does this well: put the dice in, shake it long enough that they bounce off the walls and off each other, then read them in a fixed order. Use a box with room to tumble. This is the part a hand throw has to get right and often does not.".to_owned()),
            B::Heading("4 · MORE ROLLS DO NOT HELP".to_owned()),
            B::Paragraph("The roll count is not a floor to exceed. It is the number that fills the seed: 99 fair rolls carry 255.9 of the 256 bits a 24-word seed can hold, and 50 carry 129.2 against the 128 that 12 words hold.".to_owned()),
            B::Paragraph("Rolling past that adds nothing, because the entropy target is fixed. For protocols that hash every roll it also changes the result, so extra rolls make a different seed rather than a stronger one.".to_owned()),
            B::Heading("5 · WHAT THE GUARDS DO NOT CATCH".to_owned()),
            B::Paragraph("Some protocols reject a capture when one face appears too often. That check, and Shannon-entropy meters like it, look at which faces appeared. Neither looks at the order they appeared in.".to_owned()),
            B::Verbatim("1 2 3 4 5 6 1 2 3 4 5 6 1 2 3 4 5 6 …".to_owned()),
            B::Paragraph("A repeating tape like that has a perfectly flat face distribution and passes every such guard, while being completely predictable. No automated check here establishes that your rolls were unpredictable; only your throwing method can.".to_owned()),
            B::Heading("SOURCES".to_owned()),
            B::Paragraph("M. Kapitaniak, J. Strzalko, J. Grabski and T. Kapitaniak, \"The three-dimensional dynamics of the die throw\", Chaos 22(4) 047504, 2012. doi:10.1063/1.4746038".to_owned()),
            B::Paragraph("Z. Labby, \"Weldon's Dice, Automated\", CHANCE 22(4), 2009. doi:10.1080/09332480.2009.10722977".to_owned()),
            B::Paragraph("G. Iversen, W. Longcor, F. Mosteller, J. Gilbert and C. Youtz, \"Bias and runs in dice throwing and recording\", Psychometrika 36(1), 1971. doi:10.1007/BF02291418".to_owned()),
        ],
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text() -> String {
        format!("{:?}", physical_entropy_guidance().blocks())
    }

    #[test]
    fn guidance_leads_with_the_effect_that_is_measured_to_matter() {
        let text = text();
        for expected in ["54.8%", "19.9%", "16.7%", "thorough tumble", "closed box"] {
            assert!(text.contains(expected), "missing {expected}");
        }
    }

    #[test]
    fn guidance_quantifies_bias_rather_than_warning_vaguely() {
        let text = text();
        for expected in ["255.9 bits", "253.1 bits", "198.0 bits", "1.4%"] {
            assert!(text.contains(expected), "missing {expected}");
        }
        assert!(text.contains("does not buy a better seed"));
    }

    #[test]
    fn guidance_states_that_extra_rolls_and_face_guards_prove_nothing() {
        let text = text();
        assert!(text.contains("Rolling past that adds nothing"));
        assert!(text.contains("129.2"));
        assert!(text.contains("Neither looks at the order"));
    }

    #[test]
    fn guidance_cites_every_measurement_it_quotes() {
        let text = text();
        for expected in [
            "Kapitaniak",
            "Chaos 22(4)",
            "doi:10.1063/1.4746038",
            "Labby",
            "doi:10.1080/09332480.2009.10722977",
            "Psychometrika 36(1)",
            "doi:10.1007/BF02291418",
        ] {
            assert!(text.contains(expected), "missing {expected}");
        }
    }
}
