mod asteroid;

use asteroid::Asteroid;
use hotham::{App, HothamResult};

#[cfg_attr(target_os = "android", ndk_glue::main(backtrace = "on"))]
pub fn main() {
    println!("[HOTHAM_CUBEWORLD_ANDROID] MAIN!");
    match real_main() {
        Err(e) => eprintln!("[HOTHAM_CUBEWORLD_ANDROID] - Error! {:?}", e),
        Ok(()) => println!("[HOTHAM_CUBEWORLD_ANDROID] - Finished!"),
    }
}

pub fn real_main() -> HothamResult<()> {
    let program = Asteroid::new();
    let mut app = App::new(program)?;
    app.run()?;
    Ok(())
}
