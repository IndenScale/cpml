use cpml::pipeline;
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_parse_resolve(c: &mut Criterion) {
    let input = include_str!("../samples/resource_contention.cpml");

    c.bench_function("parse_and_resolve", |b| {
        b.iter(|| {
            let doc = pipeline::parse::parse_yaml(black_box(input)).unwrap();
            let model = pipeline::resolve::resolve(doc).unwrap();
            black_box(model)
        })
    });
}

fn bench_full_pipeline_resource(c: &mut Criterion) {
    let input = include_str!("../samples/resource_contention.cpml");

    c.bench_function("full_pipeline_resource_contention", |b| {
        b.iter(|| {
            let result = pipeline::run_pipeline(black_box(input)).unwrap();
            black_box(result)
        })
    });
}

fn bench_full_pipeline_collision(c: &mut Criterion) {
    let input = include_str!("../samples/collision_demo.cpml");

    c.bench_function("full_pipeline_collision", |b| {
        b.iter(|| {
            let result = pipeline::run_pipeline(black_box(input)).unwrap();
            black_box(result)
        })
    });
}

fn bench_full_pipeline_occlusion(c: &mut Criterion) {
    let input = include_str!("../samples/occlusion_demo.cpml");

    c.bench_function("full_pipeline_occlusion", |b| {
        b.iter(|| {
            let result = pipeline::run_pipeline(black_box(input)).unwrap();
            black_box(result)
        })
    });
}

fn bench_full_pipeline_scalar(c: &mut Criterion) {
    let input = include_str!("../samples/scalar_progression.cpml");

    c.bench_function("full_pipeline_scalar_progression", |b| {
        b.iter(|| {
            let result = pipeline::run_pipeline(black_box(input)).unwrap();
            black_box(result)
        })
    });
}

fn bench_full_pipeline_rate(c: &mut Criterion) {
    let input = include_str!("../samples/ratefield_demo.cpml");

    c.bench_function("full_pipeline_ratefield", |b| {
        b.iter(|| {
            let result = pipeline::run_pipeline(black_box(input)).unwrap();
            black_box(result)
        })
    });
}

criterion_group!(
    benches,
    bench_parse_resolve,
    bench_full_pipeline_resource,
    bench_full_pipeline_collision,
    bench_full_pipeline_occlusion,
    bench_full_pipeline_scalar,
    bench_full_pipeline_rate,
);
criterion_main!(benches);
