#[cfg(target_arch = "wasm32")]
fn main() -> std::io::Result<()> {
    liptoi::browser::run()
}

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    eprintln!("Liptoi is a browser game. Run `trunk serve --open` to play it.");
}
