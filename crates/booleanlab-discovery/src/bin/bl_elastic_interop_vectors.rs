use booleanlab_discovery::elastic_interop_vectors::render_elastic_interop_vectors_v1;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    print!("{}", render_elastic_interop_vectors_v1()?);
    Ok(())
}
