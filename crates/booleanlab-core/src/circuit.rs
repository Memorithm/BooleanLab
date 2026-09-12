use core::fmt;

use crate::{BitState, StateError};

pub type NodeId = usize;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Node {
    Input(usize),
    Const(bool),
    Not(NodeId),
    And(NodeId, NodeId),
    Or(NodeId, NodeId),
    Xor(NodeId, NodeId),
    Xnor(NodeId, NodeId),
    Nand(NodeId, NodeId),
    Nor(NodeId, NodeId),
    Mux {
        select: NodeId,
        when_false: NodeId,
        when_true: NodeId,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BooleanCircuit {
    input_width: usize,
    nodes: Vec<Node>,
    outputs: Vec<NodeId>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TruthRow {
    pub input: BitState,
    pub output: BitState,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CircuitError {
    ZeroInputs,
    ZeroOutputs,
    InputWidthMismatch {
        expected: usize,
        actual: usize,
    },
    InputOutOfRange {
        node: NodeId,
        input: usize,
        width: usize,
    },
    ForwardReference {
        node: NodeId,
        referenced: NodeId,
    },
    OutputOutOfRange {
        output: usize,
        node: NodeId,
        node_count: usize,
    },
    ExactEnumerationTooWide {
        width: usize,
        maximum: usize,
    },
    State(StateError),
}

impl BooleanCircuit {
    pub const MAX_EXACT_INPUT_BITS: usize = 20;

    /// Creates and validates an acyclic Boolean circuit.
    ///
    /// # Errors
    ///
    /// Returns [`CircuitError::ZeroInputs`] or [`CircuitError::ZeroOutputs`] for
    /// empty interfaces, [`CircuitError::InputOutOfRange`] for invalid input
    /// references, [`CircuitError::ForwardReference`] when a node references
    /// itself or a later node, and [`CircuitError::OutputOutOfRange`] for an
    /// invalid output node.
    pub fn new(
        input_width: usize,
        nodes: Vec<Node>,
        outputs: Vec<NodeId>,
    ) -> Result<Self, CircuitError> {
        if input_width == 0 {
            return Err(CircuitError::ZeroInputs);
        }
        if outputs.is_empty() {
            return Err(CircuitError::ZeroOutputs);
        }

        for (node_id, node) in nodes.iter().enumerate() {
            match *node {
                Node::Input(input) => {
                    if input >= input_width {
                        return Err(CircuitError::InputOutOfRange {
                            node: node_id,
                            input,
                            width: input_width,
                        });
                    }
                }
                Node::Const(_) => {}
                Node::Not(a) => Self::validate_reference(node_id, a)?,
                Node::And(a, b)
                | Node::Or(a, b)
                | Node::Xor(a, b)
                | Node::Xnor(a, b)
                | Node::Nand(a, b)
                | Node::Nor(a, b) => {
                    Self::validate_reference(node_id, a)?;
                    Self::validate_reference(node_id, b)?;
                }
                Node::Mux {
                    select,
                    when_false,
                    when_true,
                } => {
                    Self::validate_reference(node_id, select)?;
                    Self::validate_reference(node_id, when_false)?;
                    Self::validate_reference(node_id, when_true)?;
                }
            }
        }

        for (output, &node) in outputs.iter().enumerate() {
            if node >= nodes.len() {
                return Err(CircuitError::OutputOutOfRange {
                    output,
                    node,
                    node_count: nodes.len(),
                });
            }
        }

        Ok(Self {
            input_width,
            nodes,
            outputs,
        })
    }

    #[must_use]
    pub const fn input_width(&self) -> usize {
        self.input_width
    }

    #[must_use]
    pub fn output_width(&self) -> usize {
        self.outputs.len()
    }

    #[must_use]
    pub fn nodes(&self) -> &[Node] {
        &self.nodes
    }

    /// Evaluates the circuit deterministically for one Boolean input state.
    ///
    /// # Errors
    ///
    /// Returns [`CircuitError::InputWidthMismatch`] when the supplied state has
    /// the wrong width. State access failures are propagated as
    /// [`CircuitError::State`].
    pub fn evaluate(&self, input: &BitState) -> Result<BitState, CircuitError> {
        if input.width() != self.input_width {
            return Err(CircuitError::InputWidthMismatch {
                expected: self.input_width,
                actual: input.width(),
            });
        }

        let mut values: Vec<bool> = Vec::with_capacity(self.nodes.len());
        for node in &self.nodes {
            let value = match *node {
                Node::Input(index) => input.get(index).map_err(CircuitError::State)?,
                Node::Const(value) => value,
                Node::Not(a) => !values[a],
                Node::And(a, b) => values[a] & values[b],
                Node::Or(a, b) => values[a] | values[b],
                Node::Xor(a, b) => values[a] ^ values[b],
                Node::Xnor(a, b) => !(values[a] ^ values[b]),
                Node::Nand(a, b) => !(values[a] & values[b]),
                Node::Nor(a, b) => !(values[a] | values[b]),
                Node::Mux {
                    select,
                    when_false,
                    when_true,
                } => {
                    if values[select] {
                        values[when_true]
                    } else {
                        values[when_false]
                    }
                }
            };
            values.push(value);
        }

        let mut output = BitState::zero(self.outputs.len()).map_err(CircuitError::State)?;
        for (index, &node) in self.outputs.iter().enumerate() {
            output
                .set(index, values[node])
                .map_err(CircuitError::State)?;
        }
        Ok(output)
    }

    /// Enumerates the complete truth table when the input width is bounded.
    ///
    /// # Errors
    ///
    /// Returns [`CircuitError::ExactEnumerationTooWide`] when the circuit has
    /// more than [`Self::MAX_EXACT_INPUT_BITS`] inputs. Evaluation/state errors
    /// are propagated unchanged.
    pub fn exact_truth_table(&self) -> Result<Vec<TruthRow>, CircuitError> {
        if self.input_width > Self::MAX_EXACT_INPUT_BITS {
            return Err(CircuitError::ExactEnumerationTooWide {
                width: self.input_width,
                maximum: Self::MAX_EXACT_INPUT_BITS,
            });
        }

        let row_count = 1_usize << self.input_width;
        let mut rows = Vec::with_capacity(row_count);
        for pattern in 0..row_count {
            let mut input = BitState::zero(self.input_width).map_err(CircuitError::State)?;
            for bit in 0..self.input_width {
                input
                    .set(bit, ((pattern >> bit) & 1) == 1)
                    .map_err(CircuitError::State)?;
            }
            let output = self.evaluate(&input)?;
            rows.push(TruthRow { input, output });
        }
        Ok(rows)
    }

    fn validate_reference(node: NodeId, referenced: NodeId) -> Result<(), CircuitError> {
        if referenced >= node {
            Err(CircuitError::ForwardReference { node, referenced })
        } else {
            Ok(())
        }
    }
}

impl fmt::Display for CircuitError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroInputs => {
                write!(formatter, "Boolean circuit must declare at least one input")
            }
            Self::ZeroOutputs => write!(
                formatter,
                "Boolean circuit must declare at least one output"
            ),
            Self::InputWidthMismatch { expected, actual } => {
                write!(
                    formatter,
                    "input width mismatch: expected {expected}, got {actual}"
                )
            }
            Self::InputOutOfRange { node, input, width } => write!(
                formatter,
                "node {node} references input {input}, outside input width {width}"
            ),
            Self::ForwardReference { node, referenced } => write!(
                formatter,
                "node {node} references node {referenced}, which is not strictly earlier"
            ),
            Self::OutputOutOfRange {
                output,
                node,
                node_count,
            } => write!(
                formatter,
                "output {output} references node {node}, but circuit has {node_count} nodes"
            ),
            Self::ExactEnumerationTooWide { width, maximum } => write!(
                formatter,
                "exact truth-table enumeration supports at most {maximum} inputs, got {width}"
            ),
            Self::State(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for CircuitError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evaluates_xor_and_xnor() {
        let circuit = BooleanCircuit::new(
            2,
            vec![
                Node::Input(0),
                Node::Input(1),
                Node::Xor(0, 1),
                Node::Xnor(0, 1),
            ],
            vec![2, 3],
        )
        .unwrap();

        let input = BitState::from_bools(&[true, false]).unwrap();
        let output = circuit.evaluate(&input).unwrap();
        assert_eq!(output.iter().collect::<Vec<_>>(), vec![true, false]);
    }

    #[test]
    fn rejects_forward_reference() {
        let result = BooleanCircuit::new(1, vec![Node::Not(0)], vec![0]);
        assert!(matches!(
            result,
            Err(CircuitError::ForwardReference {
                node: 0,
                referenced: 0
            })
        ));
    }

    #[test]
    fn enumerates_exact_truth_table() {
        let circuit = BooleanCircuit::new(
            2,
            vec![Node::Input(0), Node::Input(1), Node::And(0, 1)],
            vec![2],
        )
        .unwrap();
        let rows = circuit.exact_truth_table().unwrap();
        assert_eq!(rows.len(), 4);
        let outputs = rows
            .iter()
            .map(|row| row.output.get(0).unwrap())
            .collect::<Vec<_>>();
        assert_eq!(outputs, vec![false, false, false, true]);
    }
}
