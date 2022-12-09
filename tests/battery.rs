use rustww::{battery::Battery, Result};
use wasm_bindgen_test::{wasm_bindgen_test, console_log};

#[wasm_bindgen_test]
async fn test () -> Result<()> {
    let battery = Battery::new().await?;
    
    console_log!("{}", battery.level());
    Ok(())
}