use rustww::battery::Battery;

#[wasm_bindgen_test]
fn test () {
    let battery = Battery::new();
}