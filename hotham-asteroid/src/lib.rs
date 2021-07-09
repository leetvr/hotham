pub mod asteroid;

use std::{thread, time::Duration};

use asteroid::Asteroid;
use hotham::{App, HothamResult};

#[cfg_attr(target_os = "android", ndk_glue::main(backtrace = "on"))]
pub fn main() {
    println!("[HOTHAM_ASTEROID_ANDROID] MAIN!");
    match real_main() {
        Err(e) => {
            println!("[HOTHAM_ASTEROID_ANDROID] - Error! {:?}", e)
        }
        Ok(()) => println!("[HOTHAM_ASTEROID_ANDROID] - Finished!"),
    }
}

pub fn real_main() -> HothamResult<()> {
    let program = Asteroid::new();
    let mut app = App::new(program)?;
    app.run()?;
    Ok(())
}
