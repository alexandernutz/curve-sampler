mod app;
mod audio;
mod bezier;
mod curve_editor;
mod midi;
mod transform;
pub mod waveforms;
mod zebra_format;

pub use app::App;

#[cfg(not(target_arch = "wasm32"))]
pub fn run_native() {
    env_logger::init();
    let native_options = eframe::NativeOptions::default();
    eframe::run_native(
        "Zebra Curve Transform",
        native_options,
        Box::new(|cc| Ok(Box::new(App::new(cc)))),
    )
    .expect("Failed to run native app");
}

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(start)]
pub fn start() -> Result<(), JsValue> {
    use wasm_bindgen::JsCast as _;

    console_error_panic_hook::set_once();
    let _ = console_log::init_with_level(log::Level::Debug);

    let canvas = web_sys::window()
        .and_then(|w| w.document())
        .and_then(|d| d.get_element_by_id("the_canvas_id"))
        .and_then(|e| e.dyn_into::<web_sys::HtmlCanvasElement>().ok())
        .expect("canvas element 'the_canvas_id' not found");

    let web_options = eframe::WebOptions::default();

    wasm_bindgen_futures::spawn_local(async move {
        eframe::WebRunner::new()
            .start(
                canvas,
                web_options,
                Box::new(|cc| Ok(Box::new(App::new(cc)))),
            )
            .await
            .expect("failed to start eframe");
    });

    Ok(())
}
