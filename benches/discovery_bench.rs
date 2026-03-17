use criterion::{black_box, criterion_group, criterion_main, Criterion};
use http_traffic_sim::discovery::*;

fn bench_port_spec_single(c: &mut Criterion) {
    c.bench_function("port_spec_single", |b| {
        b.iter(|| {
            let spec = PortSpec::Single(black_box(8080));
            black_box(spec.to_vec())
        })
    });
}

fn bench_port_spec_list(c: &mut Criterion) {
    c.bench_function("port_spec_list", |b| {
        b.iter(|| {
            let spec = PortSpec::List(vec![80, 443, 8080, 8443]);
            black_box(spec.to_vec())
        })
    });
}

fn bench_port_spec_range_small(c: &mut Criterion) {
    c.bench_function("port_spec_range_small", |b| {
        b.iter(|| {
            let spec = PortSpec::Range {
                start: black_box(8000),
                end: black_box(8010),
            };
            black_box(spec.to_vec())
        })
    });
}

fn bench_port_spec_range_large(c: &mut Criterion) {
    c.bench_function("port_spec_range_large", |b| {
        b.iter(|| {
            let spec = PortSpec::Range {
                start: black_box(8000),
                end: black_box(9000),
            };
            black_box(spec.to_vec())
        })
    });
}

fn bench_extract_host(c: &mut Criterion) {
    c.bench_function("extract_host", |b| {
        b.iter(|| {
            extract_host_from_url(black_box("https://api.example.com:8443/health"))
        })
    });
}

criterion_group!(
    benches,
    bench_port_spec_single,
    bench_port_spec_list,
    bench_port_spec_range_small,
    bench_port_spec_range_large,
    bench_extract_host
);
criterion_main!(benches);
