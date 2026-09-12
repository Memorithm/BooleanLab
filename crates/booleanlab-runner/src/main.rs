use booleanlab_core::{BitState, BooleanCircuit, Node};

fn bits(state: &BitState) -> String {
    state
        .iter()
        .rev()
        .map(|bit| if bit { '1' } else { '0' })
        .collect()
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let circuit = BooleanCircuit::new(
        2,
        vec![
            Node::Input(0),
            Node::Input(1),
            Node::Xor(0, 1),
            Node::And(0, 1),
            Node::Or(2, 3),
        ],
        vec![2, 3, 4],
    )?;

    println!("BL-0 deterministic circuit smoke experiment");
    println!("inputs=2 outputs=3 nodes={}", circuit.nodes().len());
    println!("columns=input | xor and composite");

    for row in circuit.exact_truth_table()? {
        println!("{} | {}", bits(&row.input), bits(&row.output));
    }

    Ok(())
}
