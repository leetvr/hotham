use criterion::{criterion_group, criterion_main, Criterion};
use hotham::systems::{update_parent_transform_matrix_system, update_transform_matrix_system};
use hotham::util::get_world_with_hands;
use legion::{Resources, Schedule};

fn transform(c: &mut Criterion) {
    let mut world = get_world_with_hands();
    let mut resources = Resources::default();
    let mut schedule = Schedule::builder()
        .add_system(update_transform_matrix_system())
        .add_system(update_parent_transform_matrix_system())
        .build();
    c.bench_function("Transform", |b| {
        b.iter(|| schedule.execute(&mut world, &mut resources))
    });
}

criterion_group!(benches, transform);
criterion_main!(benches);
