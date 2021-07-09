use criterion::{criterion_group, criterion_main, Criterion};
// use hotham::model::load_models;

fn bench(c: &mut Criterion) {
    let mut group = c.benchmark_group("gltf_benches");
    group.sample_size(10);
    group.bench_function("load_models", |_| {
        // b.iter(|| {
        //     load_models(include_bytes!(
        //         "C:\\Users\\kanem\\Development\\hotham\\hotham-asteroid\\assets\\asteroid.glb"
        //     ))
        // })
    });
    group.finish();
}

criterion_group!(
    benches,
    bench,
    // glb_benchmark,
    // gltf_benchmark,
    // gltf_no_images_benchmark,
    // gltf_no_parse_images_benchmark
);
criterion_main!(benches);
