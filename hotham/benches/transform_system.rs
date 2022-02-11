// TODO: Add benchmarks back
// use criterion::{criterion_group, criterion_main, Criterion};
// use hotham::systems::{update_parent_transform_matrix_system, update_transform_matrix_system};

// fn transform(c: &mut Criterion) {
//     let mut world = get_world_with_hands();
//     let mut q1 = Default::default();
//     let mut q2 = Default::default();
//     let mut q3 = Default::default();

//     c.bench_function("Transform", |b| {
//         b.iter(|| {
//             update_transform_matrix_system(&mut q1, &mut world);
//             update_parent_transform_matrix_system(&mut q2, &mut q3, &mut world);
//         });
//     });
// }

// criterion_group!(benches, transform);
// criterion_main!(benches);
fn main() {}
