//! Export the authoritative registry for tooling and the browser-only preview.
fn main() {
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "nodes": misclab_core::nodes::default_registry().descriptors(),
            "categories": misclab_core::node::catalog::categories(),
        }))
        .unwrap()
    );
}
