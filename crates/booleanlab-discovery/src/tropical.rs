//! Exact bounded Boolean × tropical (max-plus) controls for BL-13.4.
//!
//! A tropical polynomial is evaluated over Boolean inputs embedded as exact
//! integers `x_i ∈ {0,1}`. Tropical multiplication of selected variables is
//! ordinary integer addition and tropical addition between monomials is `max`.
//! The Boolean observation boundary compares two such polynomials. This module
//! is a deterministic control generator, not a novelty or performance claim.

use std::fmt;

use crate::{BooleanFunction, FunctionError};

/// Exhaustive bound for the initial tropical control generator.
pub const MAX_TROPICAL_CONTROL_BITS: u32 = 16;

/// One max-plus monomial `constant + Σ x_i` over the variables in `variables`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TropicalMonomial {
    pub constant: i64,
    /// Bit `i` selects Boolean input variable `x_i`.
    pub variables: u64,
}

/// Exact max-plus polynomial over a fixed Boolean input width.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TropicalPolynomial {
    input_bits: u32,
    monomials: Vec<TropicalMonomial>,
}

/// Boolean observation applied to a pair of tropical polynomials.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TropicalComparison {
    Greater,
    GreaterOrEqual,
}

/// One exact Boolean predicate induced by two max-plus polynomials.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TropicalPredicate {
    left: TropicalPolynomial,
    right: TropicalPolynomial,
    comparison: TropicalComparison,
}

/// Fail-closed errors for bounded tropical controls.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TropicalError {
    ZeroInputBits,
    WidthTooLarge { width: u32, maximum: u32 },
    EmptyPolynomial,
    VariableOutOfRange { variables: u64, input_bits: u32 },
    InputOutOfRange { input: u64, input_bits: u32 },
    WidthMismatch { left: u32, right: u32 },
    ArithmeticOverflow,
    Function(FunctionError),
}

impl TropicalPolynomial {
    /// Build a bounded exact max-plus polynomial.
    ///
    /// # Errors
    ///
    /// Rejects zero/oversized widths, an empty monomial set, or variable masks
    /// that address an input outside the declared width.
    pub fn new(input_bits: u32, monomials: Vec<TropicalMonomial>) -> Result<Self, TropicalError> {
        if input_bits == 0 {
            return Err(TropicalError::ZeroInputBits);
        }
        if input_bits > MAX_TROPICAL_CONTROL_BITS {
            return Err(TropicalError::WidthTooLarge {
                width: input_bits,
                maximum: MAX_TROPICAL_CONTROL_BITS,
            });
        }
        if monomials.is_empty() {
            return Err(TropicalError::EmptyPolynomial);
        }
        let valid_mask = (1_u64 << input_bits) - 1;
        if let Some(monomial) = monomials
            .iter()
            .find(|monomial| monomial.variables & !valid_mask != 0)
        {
            return Err(TropicalError::VariableOutOfRange {
                variables: monomial.variables,
                input_bits,
            });
        }
        Ok(Self {
            input_bits,
            monomials,
        })
    }

    #[must_use]
    pub const fn input_bits(&self) -> u32 {
        self.input_bits
    }

    /// Evaluate the exact max-plus polynomial at one packed Boolean input.
    ///
    /// # Errors
    ///
    /// Rejects inputs outside the declared width and checked-integer overflow.
    pub fn evaluate(&self, input: u64) -> Result<i64, TropicalError> {
        if input >= (1_u64 << self.input_bits) {
            return Err(TropicalError::InputOutOfRange {
                input,
                input_bits: self.input_bits,
            });
        }

        self.monomials
            .iter()
            .map(|monomial| {
                let active = (input & monomial.variables).count_ones();
                monomial
                    .constant
                    .checked_add(i64::from(active))
                    .ok_or(TropicalError::ArithmeticOverflow)
            })
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .max()
            .ok_or(TropicalError::EmptyPolynomial)
    }
}

impl TropicalPredicate {
    /// Bind two same-width tropical polynomials to a Boolean comparison.
    ///
    /// # Errors
    ///
    /// Rejects mismatched input widths.
    pub fn new(
        left: TropicalPolynomial,
        right: TropicalPolynomial,
        comparison: TropicalComparison,
    ) -> Result<Self, TropicalError> {
        if left.input_bits() != right.input_bits() {
            return Err(TropicalError::WidthMismatch {
                left: left.input_bits(),
                right: right.input_bits(),
            });
        }
        Ok(Self {
            left,
            right,
            comparison,
        })
    }

    #[must_use]
    pub const fn input_bits(&self) -> u32 {
        self.left.input_bits()
    }

    /// Evaluate the declared Boolean observation boundary.
    ///
    /// # Errors
    ///
    /// Propagates exact polynomial-evaluation failures.
    pub fn evaluate(&self, input: u64) -> Result<bool, TropicalError> {
        let left = self.left.evaluate(input)?;
        let right = self.right.evaluate(input)?;
        Ok(match self.comparison {
            TropicalComparison::Greater => left > right,
            TropicalComparison::GreaterOrEqual => left >= right,
        })
    }

    /// Exhaustively materialize the induced Boolean truth table.
    ///
    /// # Errors
    ///
    /// Propagates tropical evaluation failures and Boolean-function validation.
    pub fn boolean_function(&self) -> Result<BooleanFunction, TropicalError> {
        let rows = 1_u64 << self.input_bits();
        let capacity = usize::try_from(rows).map_err(|_| TropicalError::ArithmeticOverflow)?;
        let mut table = Vec::with_capacity(capacity);
        for input in 0..rows {
            table.push(u8::from(self.evaluate(input)?));
        }
        BooleanFunction::new(self.input_bits(), table).map_err(TropicalError::Function)
    }
}

/// Frozen BL-13.4.0 control family.
///
/// The four comparisons deliberately mix singleton and multi-variable tropical
/// monomials and both strict/non-strict observation boundaries. They are small
/// enough for complete truth-table evaluation and are controls only.
///
/// # Errors
///
/// Returns a fail-closed tropical construction error if any frozen polynomial
/// violates the bounded exact representation contract.
pub fn bl13_4_control_predicates() -> Result<Vec<TropicalPredicate>, TropicalError> {
    const N: u32 = 4;
    let polynomial = |terms: &[(i64, u64)]| {
        TropicalPolynomial::new(
            N,
            terms
                .iter()
                .map(|&(constant, variables)| TropicalMonomial {
                    constant,
                    variables,
                })
                .collect(),
        )
    };

    [
        (
            polynomial(&[(0, 0b0011), (1, 0b0100)])?,
            polynomial(&[(0, 0b0110), (1, 0b0001)])?,
            TropicalComparison::Greater,
        ),
        (
            polynomial(&[(-1, 0b1010), (0, 0b0101)])?,
            polynomial(&[(-1, 0b1100), (0, 0b0011)])?,
            TropicalComparison::GreaterOrEqual,
        ),
        (
            polynomial(&[(0, 0b1001), (0, 0b0110), (1, 0)])?,
            polynomial(&[(0, 0b0101), (0, 0b1010), (1, 0)])?,
            TropicalComparison::Greater,
        ),
        (
            polynomial(&[(-2, 0b1111), (0, 0b0010), (0, 0b1000)])?,
            polynomial(&[(-1, 0b0111), (0, 0b0100)])?,
            TropicalComparison::GreaterOrEqual,
        ),
    ]
    .into_iter()
    .map(|(left, right, comparison)| TropicalPredicate::new(left, right, comparison))
    .collect()
}

impl fmt::Display for TropicalError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroInputBits => {
                write!(formatter, "tropical control input width must be non-zero")
            }
            Self::WidthTooLarge { width, maximum } => write!(
                formatter,
                "tropical control supports at most {maximum} input bits, got {width}"
            ),
            Self::EmptyPolynomial => {
                write!(formatter, "tropical polynomial must contain a monomial")
            }
            Self::VariableOutOfRange {
                variables,
                input_bits,
            } => write!(
                formatter,
                "tropical variable mask {variables:#x} exceeds declared {input_bits}-bit input width"
            ),
            Self::InputOutOfRange { input, input_bits } => write!(
                formatter,
                "tropical input {input:#x} exceeds declared {input_bits}-bit input width"
            ),
            Self::WidthMismatch { left, right } => write!(
                formatter,
                "tropical predicate width mismatch: left {left}, right {right}"
            ),
            Self::ArithmeticOverflow => write!(formatter, "tropical integer evaluation overflowed"),
            Self::Function(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for TropicalError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DedupIndex, DedupOutcome};

    #[test]
    fn hand_computable_two_bit_polynomial_is_exact() {
        let polynomial = TropicalPolynomial::new(
            2,
            vec![
                TropicalMonomial {
                    constant: 0,
                    variables: 0b01,
                },
                TropicalMonomial {
                    constant: -1,
                    variables: 0b11,
                },
            ],
        )
        .unwrap();
        assert_eq!(
            (0..4)
                .map(|input| polynomial.evaluate(input).unwrap())
                .collect::<Vec<_>>(),
            vec![0, 1, 0, 1]
        );
    }

    #[test]
    fn control_family_is_deterministic_exact_and_nonconstant() {
        let first = bl13_4_control_predicates().unwrap();
        let second = bl13_4_control_predicates().unwrap();
        assert_eq!(first, second);
        assert_eq!(first.len(), 4);

        let functions = first
            .iter()
            .map(TropicalPredicate::boolean_function)
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        let mut index = DedupIndex::new();
        for function in functions {
            let ones = function
                .truth_table()
                .iter()
                .map(|&value| usize::from(value))
                .sum::<usize>();
            assert!(ones > 0 && ones < function.truth_table().len());
            assert_eq!(index.insert(function), DedupOutcome::New);
        }
        assert_eq!(index.unique_functions(), 4);
    }

    #[test]
    fn malformed_bounds_fail_closed() {
        assert_eq!(
            TropicalPolynomial::new(
                0,
                vec![TropicalMonomial {
                    constant: 0,
                    variables: 0
                }]
            ),
            Err(TropicalError::ZeroInputBits)
        );
        assert_eq!(
            TropicalPolynomial::new(
                2,
                vec![TropicalMonomial {
                    constant: 0,
                    variables: 0b100
                }]
            ),
            Err(TropicalError::VariableOutOfRange {
                variables: 0b100,
                input_bits: 2,
            })
        );
        let polynomial = TropicalPolynomial::new(
            2,
            vec![TropicalMonomial {
                constant: i64::MAX,
                variables: 1,
            }],
        )
        .unwrap();
        assert_eq!(
            polynomial.evaluate(1),
            Err(TropicalError::ArithmeticOverflow)
        );
    }
}
