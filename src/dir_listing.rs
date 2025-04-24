use super::models::{Cli, FileEntry};
use super::utils::{human_readable_size, progress_bar_init};
use indicatif::ProgressBar;
use rayon::prelude::*;
use std::fs;
use std::path::Path;

pub fn calculate_dir_size(
    path: &Path,
    human_readable: bool,
    main_pb: &ProgressBar,
    parallel: bool,
) -> (u64, String) {
    fn inner_calculate(p: &Path, pb: &ProgressBar, parallel: bool) -> u64 {
        fs::read_dir(p)
            .map(|entries| {
                let base_iter = entries.filter_map(|e| {
                    pb.tick();
                    e.ok()
                });
                if parallel {
                    let processed_iter = base_iter.par_bridge();
                    processed_iter.map(|e| process_entry(e, pb, parallel)).sum()
                } else {
                    base_iter.map(|e| process_entry(e, pb, parallel)).sum()
                }
                // 统一使用并行迭代器接口
            })
            .unwrap_or(0)
    }

    // 新增辅助函数处理条目
    fn process_entry(e: std::fs::DirEntry, pb: &ProgressBar, parallel: bool) -> u64 {
        let md = match e.metadata() {
            Ok(metadata) => metadata,
            Err(_) => return 0,
        };
        if md.is_dir() {
            inner_calculate(&e.path(), pb, parallel)
        } else {
            md.len()
        }
    }

    main_pb.set_message(format!("计算 {}...", path.display()));
    let total = inner_calculate(path, main_pb, parallel);
    main_pb.set_message("处理中...");

    let converted = if human_readable {
        human_readable_size(total)
    } else {
        total.to_string()
    };
    (total, converted)
}
pub fn list_directory(path: &Path, args: &Cli) {
    let entries = match fs::read_dir(path) {
        Ok(entries) => entries,
        Err(e) => {
            eprintln!("ls: cannot access '{}': {}", path.display(), e);
            return;
        }
    };

    let mut files: Vec<String> = Vec::new();

    for entry in entries.flatten() {
        let file_name = entry.file_name().to_string_lossy().to_string();
        files.push(file_name);
    }

    files.sort();
    let scan_pb = progress_bar_init(None).unwrap();

    let mut entries = Vec::new(); // 新增存储条目信息的结构

    if args.long_format {
        let process_pb = progress_bar_init(None).unwrap(); // 修改为不传入具体数值
        process_pb.set_message("处理中..."); // 设置固定提示信息

        for (i, file) in files.iter().enumerate() {
            // process_pb.finish_and_clear(); // 注释此行
            process_pb.tick();
            let file_path = path.join(&file);
            let metadata = match file_path.metadata() {
                Ok(m) => m,
                Err(e) => {
                    eprintln!("ls: cannot access '{}': {}", file_path.display(), e);
                    continue;
                }
            };
            let (size_display, size_raw) = if metadata.is_dir() {
                let (raw, converted) =
                    calculate_dir_size(&file_path, args.human_readable, &process_pb, args.parallel);
                (converted, raw)
            } else if args.human_readable {
                (human_readable_size(metadata.len()), metadata.len())
            } else {
                (metadata.len().to_string(), metadata.len())
            };
            entries.push(FileEntry {
                file_type: if metadata.is_dir() { 'd' } else { '-' },
                permissions: format!(
                    "{}-{}-{}",
                    if metadata.permissions().readonly() {
                        "r"
                    } else {
                        " "
                    },
                    "w",
                    "x"
                ),
                size_display,
                size_raw,
                path: file_path.display().to_string(),
            });
        }

        process_pb.finish_and_clear();
        let mut sum_size = 0;
        for entry in &entries {
            sum_size += entry.size_raw; // 使用第4个字段的原始大小
        }
        // 在打印循环前添加
        println!("{:<5} {:<10} {:<10} {:<20}", "类型", "权限", "大小", "路径");
        for entry in entries.iter() {
            println!(
                "{:<5} {:<10} {:<10} {:<20}",
                entry.file_type, entry.permissions, entry.size_display, entry.path
            );
        }
        println!(
            "\n总数量: {} 个条目 | 总大小: {}",
            entries.len(),
            human_readable_size(sum_size)
        );
    } else {
        for file in files {
            println!("{}", file);
        }
    }
    scan_pb.finish_and_clear(); // 完成后清理进度条
}
