//! Versioned exact small-domain Boolean vectors for cross-repository consumers.
//!
//! `BooleanLab` owns the mathematical test vectors and their exact truth-table
//! semantics. Consumers such as `ElasticXxx` may snapshot these vectors into
//! tests, but this module contains no `ElasticXxx` runtime dependency and grants
//! no validation or actuation authority.

use std::fmt::Write as _;

use crate::{BooleanFunction, FunctionError};

/// Current schema of the emitted TSV fixture.
pub const ELASTIC_INTEROP_VECTOR_SCHEMA_V1: u16 = 1;
/// Number of input bits used by the first interop fixture family.
pub const ELASTIC_INTEROP_INPUT_BITS_V1: u32 = 3;
/// Maximum RPN tokens accepted by the bounded exact fixture evaluator.
pub const MAX_INTEROP_EXPRESSION_TOKENS: usize = 32;

const CASES_V1: [(&str, &str, &str); 7] = [
    (
        "conjunction-with-negation",
        "conjunction",
        "p0 p1 not and p2 and",
    ),
    (
        "disjunction-with-negation",
        "disjunction",
        "p0 p1 not or p2 or",
    ),
    ("xor-two-inputs", "general", "p0 p1 xor"),
    ("implication", "general", "p0 p1 implies"),
    ("tautology", "tautology", "p0 p0 not or"),
    ("contradiction", "contradiction", "p0 p0 not and"),
    ("nested-xor-implies", "general", "p0 p1 xor p2 implies"),
];

/// Exact whole-function logical classification.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ElasticInteropLogicalClassV1 {
    Contradiction,
    Contingent,
    Tautology,
}

impl ElasticInteropLogicalClassV1 {
    #[must_use]
    pub const fn satisfiable(self) -> bool {
        !matches!(self, Self::Contradiction)
    }

    #[must_use]
    pub const fn tautology(self) -> bool {
        matches!(self, Self::Tautology)
    }

    #[must_use]
    pub const fn contradiction(self) -> bool {
        matches!(self, Self::Contradiction)
    }
}

/// One exact versioned vector row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ElasticInteropVectorV1 {
    pub case_id: &'static str,
    pub shape_class: &'static str,
    pub input_bits: u32,
    pub expression_rpn: &'static str,
    pub truth_table: Vec<u8>,
    pub algebraic_degree: u32,
    pub nonlinearity: u64,
    pub balanced: bool,
    pub correlation_immunity: u32,
    pub logical_class: ElasticInteropLogicalClassV1,
    pub booleanlab_fingerprint: u64,
}

/// Errors in the bounded cross-repository fixture grammar.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ElasticInteropVectorError {
    TooManyTokens { maximum: usize },
    UnknownToken(String),
    PredicateOutOfRange { predicate: u32, input_bits: u32 },
    StackUnderflow { token: String },
    InvalidFinalStackDepth { depth: usize },
    Function(FunctionError),
    Formatting,
}

impl std::fmt::Display for ElasticInteropVectorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooManyTokens { maximum } => {
                write!(f, "interop expression exceeds {maximum} tokens")
            }
            Self::UnknownToken(token) => write!(f, "unknown interop RPN token {token:?}"),
            Self::PredicateOutOfRange {
                predicate,
                input_bits,
            } => write!(
                f,
                "interop predicate p{predicate} is outside {input_bits}-bit domain"
            ),
            Self::StackUnderflow { token } => {
                write!(f, "interop RPN stack underflow at token {token:?}")
            }
            Self::InvalidFinalStackDepth { depth } => {
                write!(f, "interop RPN expression ended with stack depth {depth}")
            }
            Self::Function(error) => error.fmt(f),
            Self::Formatting => write!(f, "failed to format interop fixture"),
        }
    }
}

impl std::error::Error for ElasticInteropVectorError {}

impl From<FunctionError> for ElasticInteropVectorError {
    fn from(value: FunctionError) -> Self {
        Self::Function(value)
    }
}

fn pop(stack: &mut Vec<bool>, token: &str) -> Result<bool, ElasticInteropVectorError> {
    stack
        .pop()
        .ok_or_else(|| ElasticInteropVectorError::StackUnderflow {
            token: token.to_owned(),
        })
}

fn evaluate_rpn(
    expression: &str,
    input_bits: u32,
    assignment: u64,
) -> Result<bool, ElasticInteropVectorError> {
    let tokens = expression.split_whitespace().collect::<Vec<_>>();
    if tokens.len() > MAX_INTEROP_EXPRESSION_TOKENS {
        return Err(ElasticInteropVectorError::TooManyTokens {
            maximum: MAX_INTEROP_EXPRESSION_TOKENS,
        });
    }
    let mut stack = Vec::with_capacity(tokens.len());
    for token in tokens {
        match token {
            "true" => stack.push(true),
            "false" => stack.push(false),
            "not" => {
                let value = pop(&mut stack, token)?;
                stack.push(!value);
            }
            "and" | "or" | "xor" | "implies" => {
                let rhs = pop(&mut stack, token)?;
                let lhs = pop(&mut stack, token)?;
                stack.push(match token {
                    "and" => lhs && rhs,
                    "or" => lhs || rhs,
                    "xor" => lhs ^ rhs,
                    "implies" => !lhs || rhs,
                    _ => unreachable!("matched operator"),
                });
            }
            predicate if predicate.starts_with('p') => {
                let index = predicate[1..]
                    .parse::<u32>()
                    .map_err(|_| ElasticInteropVectorError::UnknownToken(predicate.to_owned()))?;
                if index >= input_bits {
                    return Err(ElasticInteropVectorError::PredicateOutOfRange {
                        predicate: index,
                        input_bits,
                    });
                }
                stack.push(((assignment >> index) & 1) != 0);
            }
            _ => return Err(ElasticInteropVectorError::UnknownToken(token.to_owned())),
        }
    }
    if stack.len() != 1 {
        return Err(ElasticInteropVectorError::InvalidFinalStackDepth { depth: stack.len() });
    }
    Ok(stack[0])
}

fn vector(
    case_id: &'static str,
    shape_class: &'static str,
    expression_rpn: &'static str,
) -> Result<ElasticInteropVectorV1, ElasticInteropVectorError> {
    let function = BooleanFunction::from_fn(ELASTIC_INTEROP_INPUT_BITS_V1, |assignment| {
        evaluate_rpn(expression_rpn, ELASTIC_INTEROP_INPUT_BITS_V1, assignment)
            .expect("static interop expression is validated by repository tests")
    })?;
    let metrics = function.exact_metrics();
    let logical_class = if function.truth_table().iter().all(|value| *value == 1) {
        ElasticInteropLogicalClassV1::Tautology
    } else if function.truth_table().iter().all(|value| *value == 0) {
        ElasticInteropLogicalClassV1::Contradiction
    } else {
        ElasticInteropLogicalClassV1::Contingent
    };
    Ok(ElasticInteropVectorV1 {
        case_id,
        shape_class,
        input_bits: ELASTIC_INTEROP_INPUT_BITS_V1,
        expression_rpn,
        truth_table: function.truth_table().to_vec(),
        algebraic_degree: metrics.algebraic_degree,
        nonlinearity: metrics.nonlinearity,
        balanced: metrics.balanced,
        correlation_immunity: metrics.correlation_immunity,
        logical_class,
        booleanlab_fingerprint: function.stable_fingerprint(),
    })
}

/// Generate the canonical V1 exact fixture in stable case order.
///
/// # Errors
///
/// Returns a bounded grammar or exact-function construction error if a static
/// vector definition becomes invalid.
pub fn elastic_interop_vectors_v1() -> Result<Vec<ElasticInteropVectorV1>, ElasticInteropVectorError>
{
    CASES_V1
        .iter()
        .map(|(case_id, shape_class, expression)| vector(case_id, shape_class, expression))
        .collect()
}

fn bits(table: &[u8]) -> String {
    table
        .iter()
        .map(|value| char::from(b'0' + *value))
        .collect()
}

/// Render the canonical V1 fixture as deterministic TSV.
///
/// The truth-table string is row-major: character `x` is `f(x)`, with `p0`
/// bound to the least-significant assignment bit.
///
/// # Errors
///
/// Returns any vector-generation or formatting error.
pub fn render_elastic_interop_vectors_v1() -> Result<String, ElasticInteropVectorError> {
    let mut output = String::new();
    output.push_str("schema_version\tcase_id\tshape_class\tinput_bits\texpression_rpn\ttruth_table_row_major\talgebraic_degree\tnonlinearity\tbalanced\tcorrelation_immunity\tsatisfiable\ttautology\tcontradiction\tbooleanlab_fingerprint\n");
    for case in elastic_interop_vectors_v1()? {
        writeln!(
            output,
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{:016x}",
            ELASTIC_INTEROP_VECTOR_SCHEMA_V1,
            case.case_id,
            case.shape_class,
            case.input_bits,
            case.expression_rpn,
            bits(&case.truth_table),
            case.algebraic_degree,
            case.nonlinearity,
            case.balanced,
            case.correlation_immunity,
            case.logical_class.satisfiable(),
            case.logical_class.tautology(),
            case.logical_class.contradiction(),
            case.booleanlab_fingerprint,
        )
        .map_err(|_| ElasticInteropVectorError::Formatting)?;
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    const COMMITTED_V1: &str = include_str!("../../../interop/elasticxxx/exact-vectors-v1.tsv");

    #[test]
    fn committed_fixture_is_exactly_regenerated() {
        assert_eq!(render_elastic_interop_vectors_v1().unwrap(), COMMITTED_V1);
    }

    #[test]
    fn rows_cover_fast_shapes_general_shapes_and_sat_boundaries() {
        let rows = elastic_interop_vectors_v1().unwrap();
        assert!(rows.iter().any(|row| row.shape_class == "conjunction"));
        assert!(rows.iter().any(|row| row.shape_class == "disjunction"));
        assert!(rows.iter().any(|row| row.shape_class == "general"));
        assert!(rows.iter().any(|row| row.logical_class.tautology()));
        assert!(rows.iter().any(|row| row.logical_class.contradiction()));
        assert!(
            rows.iter().all(|row| {
                !(row.logical_class.tautology() && row.logical_class.contradiction())
            })
        );
        assert!(
            rows.iter().all(|row| {
                row.logical_class.satisfiable() != row.logical_class.contradiction()
            })
        );
    }

    #[test]
    fn row_order_binds_p0_to_least_significant_assignment_bit() {
        let xor = elastic_interop_vectors_v1()
            .unwrap()
            .into_iter()
            .find(|row| row.case_id == "xor-two-inputs")
            .unwrap();
        assert_eq!(xor.truth_table, vec![0, 1, 1, 0, 0, 1, 1, 0]);
    }

    #[test]
    fn bounded_rpn_parser_rejects_invalid_input() {
        assert_eq!(
            evaluate_rpn("p3", 3, 0),
            Err(ElasticInteropVectorError::PredicateOutOfRange {
                predicate: 3,
                input_bits: 3,
            })
        );
        assert!(matches!(
            evaluate_rpn("p0 and", 3, 0),
            Err(ElasticInteropVectorError::StackUnderflow { .. })
        ));
        assert!(matches!(
            evaluate_rpn("p0 p1", 3, 0),
            Err(ElasticInteropVectorError::InvalidFinalStackDepth { depth: 2 })
        ));
        assert!(matches!(
            evaluate_rpn("wat", 3, 0),
            Err(ElasticInteropVectorError::UnknownToken(_))
        ));
    }
}
