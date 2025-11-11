use criterion::{criterion_group, criterion_main, Criterion};
use ll::dir_listing::list_directory;
use ll::models::Cli;
use std::hint::black_box;
use std::path::Path;
fn bench_directory_listing(c: &mut Criterion) {
    let path = Path::new("C:\\Users\\hover\\AppData"); // 替换为你要测试的目录
    let args: Cli = Cli {
        file: "C:\\Users\\hover\\AppData".to_string(),
        long_format: true,
        human_readable: true,
        all: true,
        show_time: true,
        parallel: true,
        sort: true,
        name: Some("".to_string()),
        full_path: false,
    };
    println!("开始");
    c.bench_function("list_directory", |b| {
        b.iter(|| {
            // 调用你的目录列表函数
            list_directory(black_box(path), black_box(&args))
        })
    });
}

criterion_group!(benches, bench_directory_listing);
criterion_main!(benches);
