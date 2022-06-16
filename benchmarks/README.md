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

## Many vertices
### Methodology
- Start with a mesh with two triangles.
- Each second, increase the number of vertices
- Record FPS vs Vertices
- At the moment of crash, record the number of vertices and last FPS

### Hotham 0.2
- **Result**: Crash - segfault
- **Max vertices**: 1,040,400 vertices, 
- **FPS at time of crash**: 16
- **OVR stats at time of crash**: `FPS=16/72,Prd=83ms,Tear=2,Early=2,Stale=56,VSnc=0,Lat=-1,Fov=0,CPU4/GPU=4/3,1478/490MHz,OC=FF,TA=0/0/0,SP=N/N/N,Mem=1353MHz,Free=3200MB,PLS=0,Temp=31.0C/0.0C,TW=8.91ms,App=11.80ms,GD=0.00ms,CPU&GPU=58.12ms,LCnt=1,GPU%=1.00,CPU%=0.02(W0.05),DSF=1.00`
- **Highest vertex count at 72 FPS**: 144,400

## Sponza
### Methodology
- Download the [new Sponza scene](https://www.intel.com/content/www/us/en/developer/topic-technology/graphics-research/samples.html)
- Export the glTF file into a GLB file with Blender
- Place the glTF file in the `test_assets` directory.
- Record the FPS if the scene loads

### Hotham 0.2
- **Result**: Crash - `ERROR_OUT_OF_POOL_MEMORY`
- **Note**: Was unable to run in simulator so did not attempt to run in headset. Will require further investigation to load the GLB from the device's internal storage as it is too large to either include in the binary or the APK.