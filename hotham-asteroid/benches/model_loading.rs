use criterion::{criterion_group, criterion_main, Criterion};
use hotham_asteroid::asteroid::load_model_from_gltf_optimized;

fn bench(c: &mut Criterion) {
    let mut group = c.benchmark_group("gltf_benches");
    group.sample_size(10);
    group.bench_function("load_model_from_gltf_optimized", |b| {
        b.iter(|| load_model_from_gltf_optimized())
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
