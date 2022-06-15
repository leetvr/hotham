# Benchmarks

## Many objects
### Methodology
- Each second, add a cube (in this case, the default Blender cube, exported to glTF)
- At the moment of crash, record the number of cubes and current FPS

### Hotham 0.2
- **Result**: Crash - `Result::unwrap()` on an `Err` value: ERROR_OUT_OF_POOL_MEMORY', hotham\src\gltf_loader.rs:448:14`
- **Max cubes**: 98
- **FPS at time of crash**: 72
- **OVR stats**: `FPS=72/72,Prd=28ms,Tear=0,Early=0,Stale=1,VSnc=0,Lat=-1,Fov=0,CPU4/GPU=2/3,1171/490MHz,OC=FF,TA=0/0/0,SP=N/N/N,Mem=1353MHz,Free=3274MB,PLS=0,Temp=28.0C/0.0C,TW=2.39ms,App=1.73ms,GD=0.27ms,CPU&GPU=4.08ms,LCnt=1,GPU%=0.35,CPU%=0.12(W0.14),DSF=1.00`