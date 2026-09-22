// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The claim-strength vocabulary Engineering Assurance owns (FR-022).
//!
//! A claim strength names the kind of support behind one advanced verification
//! result: whether the claim was proven, checked exhaustively within a stated
//! bound, exercised by tests, or only observed. Every advanced result carries
//! exactly one strength.
//!
//! The four strengths are **unordered**. They answer "what kind of support is
//! this", not "how much", so this type deliberately implements neither `Ord`
//! nor `PartialOrd`, has no declared numeric representation and has no
//! `Default`.
//! A consumer that wants to rank, compare or promote one strength over another
//! is building a trust score, which this vocabulary does not provide. The
//! declaration order of the variants and of [`ClaimStrength::ALL`] is a listing
//! order only; discriminants are not part of the contract.
//!
//! The module depends on `serde` alone, so a consumer can enable the narrow
//! `claim-strength` feature without accepting the `full` feature's
//! `serde_json/arbitrary_precision` flip.

use std::{error::Error, fmt, str::FromStr};

use serde::{Deserialize, Serialize};

/// The kind of support behind one advanced verification result.
///
/// Serialized as its kebab-case wire name (see [`ClaimStrength::wire_name`]).
/// Unknown wire names are refused on decode and on [`FromStr`].
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ClaimStrength {
    /// The claim is established by a proof over every case it covers.
    Proven,
    /// The claim was checked exhaustively, but only within a stated bound.
    BoundedChecked,
    /// The claim was exercised by tests over a selected set of cases.
    Tested,
    /// The claim was seen to hold in observed behavior, without a check.
    Observed,
}

impl ClaimStrength {
    /// Every claim strength, in listing order.
    ///
    /// The order is for stable iteration and rendering only; it is not a rank.
    pub const ALL: [Self; 4] = [
        Self::Proven,
        Self::BoundedChecked,
        Self::Tested,
        Self::Observed,
    ];

    /// The exact wire spelling of this strength.
    #[must_use]
    pub const fn wire_name(self) -> &'static str {
        match self {
            Self::Proven => "proven",
            Self::BoundedChecked => "bounded-checked",
            Self::Tested => "tested",
            Self::Observed => "observed",
        }
    }
}

impl fmt::Display for ClaimStrength {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.wire_name())
    }
}

impl FromStr for ClaimStrength {
    type Err = UnknownClaimStrength;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|strength| strength.wire_name() == value)
            .ok_or_else(|| UnknownClaimStrength {
                value: value.to_owned(),
            })
    }
}

/// A label that is not exactly one of the declared claim-strength wire names.
///
/// Matching is exact: no case folding, trimming or aliasing is applied.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UnknownClaimStrength {
    value: String,
}

impl UnknownClaimStrength {
    /// The refused label, as supplied.
    #[must_use]
    pub fn value(&self) -> &str {
        &self.value
    }
}

impl fmt::Display for UnknownClaimStrength {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "unknown claim strength {:?}", self.value)
    }
}

impl Error for UnknownClaimStrength {}

/// Compile-time proof that no ordering is exposed.
///
/// This is the trait-absence probe `static_assertions::assert_not_impl_any!`
/// uses, written out so no dependency is added. `probe` resolves to the single
/// blanket implementation when `ClaimStrength` implements neither `PartialOrd`
/// nor `Ord`; if either is ever derived, a second implementation applies, the
/// inferred parameter becomes ambiguous, and the crate stops compiling.
/// (`Ord` requires `PartialOrd`, so the `PartialOrd` arm alone would catch both;
/// both are listed so the intent is legible.)
const _: fn() = || {
    trait AmbiguousIfOrdered<Marker> {
        fn probe() {}
    }
    impl<T: ?Sized> AmbiguousIfOrdered<()> for T {}
    struct PartialOrdMarker;
    impl<T: ?Sized + PartialOrd> AmbiguousIfOrdered<PartialOrdMarker> for T {}
    struct OrdMarker;
    impl<T: ?Sized + Ord> AmbiguousIfOrdered<OrdMarker> for T {}
    let _ = <ClaimStrength as AmbiguousIfOrdered<_>>::probe;
};

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use ix_trace_rs::trace;

    use super::{ClaimStrength, UnknownClaimStrength};

    #[test]
    #[trace("TC-150", "FR-022-AC-1")]
    fn tc_150_every_strength_has_its_exact_wire_name_and_parses_back() {
        let expected = [
            (ClaimStrength::Proven, "proven"),
            (ClaimStrength::BoundedChecked, "bounded-checked"),
            (ClaimStrength::Tested, "tested"),
            (ClaimStrength::Observed, "observed"),
        ];
        for (strength, wire) in expected {
            assert_eq!(strength.wire_name(), wire);
            assert_eq!(strength.to_string(), wire);
            assert_eq!(wire.parse::<ClaimStrength>(), Ok(strength));
        }
    }

    #[test]
    #[trace("TC-151", "FR-022-AC-2")]
    fn tc_151_unknown_or_near_miss_labels_are_refused_with_a_typed_error() {
        for label in [
            "",
            "Proven",
            "PROVEN",
            " proven",
            "proven ",
            "bounded_checked",
            "boundedChecked",
            "bounded checked",
            "verified",
            "checked",
            "strong",
            "0",
        ] {
            let error = label
                .parse::<ClaimStrength>()
                .expect_err("a label outside the vocabulary must be refused");
            assert_eq!(
                error,
                UnknownClaimStrength {
                    value: label.to_owned()
                }
            );
            assert_eq!(error.value(), label);
        }
    }

    #[test]
    #[trace("TC-152", "FR-022-AC-3")]
    fn tc_152_all_lists_every_variant_exactly_once() {
        // A new variant makes this match non-exhaustive, so the variant list
        // beside it has to be edited in the same change; the set comparison
        // then fails until `ALL` is edited too.
        let every_variant = [
            ClaimStrength::Proven,
            ClaimStrength::BoundedChecked,
            ClaimStrength::Tested,
            ClaimStrength::Observed,
        ];
        for strength in every_variant {
            match strength {
                ClaimStrength::Proven
                | ClaimStrength::BoundedChecked
                | ClaimStrength::Tested
                | ClaimStrength::Observed => {}
            }
        }
        let listed: HashSet<_> = ClaimStrength::ALL.into_iter().collect();
        assert_eq!(
            listed.len(),
            ClaimStrength::ALL.len(),
            "ALL repeats a strength"
        );
        assert_eq!(listed, every_variant.into_iter().collect::<HashSet<_>>());

        let wire_names: HashSet<_> = ClaimStrength::ALL
            .into_iter()
            .map(ClaimStrength::wire_name)
            .collect();
        assert_eq!(
            wire_names.len(),
            ClaimStrength::ALL.len(),
            "two strengths share a wire name"
        );
    }
}
