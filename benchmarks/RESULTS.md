# Benchmarks

## Many objects
### Methodology
- Each second, add a cube (in this case, the default Blender cube, exported to glTF)
- At the moment of crash, record the number of cubes and current FPS

### Hotham 0.2
- **Result**: Crash - `Result::unwrap()` on an `Err` value: ERROR_OUT_OF_POOL_MEMORY', hotham\src\gltf_loader.rs:448:14`
- **Max cubes**: 98
- **FPS at time of crash**: 72