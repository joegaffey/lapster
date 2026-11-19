use wasm_bindgen::prelude::*;

#[wasm_bindgen(start)]
pub fn start() {
    console_error_panic_hook::set_once();
}

#[wasm_bindgen]
pub fn compile_glb_track(track_json: &[u8], glb: &[u8], id: &str) -> Result<Vec<u8>, JsValue> {
    track_pack::bake_glb_track_bytes(track_json, glb, id)
        .map_err(|error| JsValue::from_str(&error.to_string()))
}
