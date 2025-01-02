pub mod sdk;
pub mod state_transitions;
pub mod verify;

#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

// TODO: Remove tracing or use
//https://github.com/old-storyai/tracing-wasm
//https://github.com/jquesada2016/tracing_subscriber_wasm
//https://crates.io/crates/tracing-web
