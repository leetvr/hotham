mod cubeworld;

use cubeworld::Cubeworld;
use hotham::{App, HothamResult};

#[cfg_attr(target_os = "android", ndk_glue::main(backtrace = "on"))]
pub fn main() -> HothamResult<()> {
    real_main()
}

pub fn real_main() -> HothamResult<()> {
    let program = Cubeworld::new();
    let mut app = App::new(program)?;
    app.run()?;
    Ok(())
}
