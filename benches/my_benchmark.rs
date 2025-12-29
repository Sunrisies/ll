use criterion::{criterion_group, criterion_main, Criterion};
use ll::dir_listing::list_directory;
use ll::models::Cli;
use std::hint::black_box;
use std::path::Path;
fn bench_directory_listing(c: &mut Criterion) {
    let path = Path::new("d:/project/github/user/ll/src");
    let args: Cli = Cli {
        file: "d:/project/github/user/ll/src".to_string(),
        long_format: true,
        human_readable: true,
        all: true,
        show_time: true,
        parallel: true,
        sort: true,
        name: None, // 移除name参数，测试正常列表功能
        full_path: false,
    };
    println!("开始");

    c.bench_function("list_directory_v1", |b| {
        b.iter(|| list_directory(black_box(path), black_box(&args)))
    });
}

criterion_group!(benches, bench_directory_listing);
criterion_main!(benches);
