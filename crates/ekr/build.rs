//! Native-only binding generation: installing the CLI does not build browser Rust.
fn main() {
    println!("cargo:rerun-if-changed=assets/search.wasm");
    let out = std::env::var_os("OUT_DIR").expect("Cargo output directory");
    wasm_bindgen_cli_support::Bindgen::new()
        .input_path("assets/search.wasm")
        .out_name("search")
        .typescript(false)
        .deno(true)
        .expect("pinned automatic module target")
        .generate(out)
        .expect("generate embedded browser interop from checked Rust WASM");
}
