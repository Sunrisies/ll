use super::models::{Cli, FileEntry};
use super::utils::{human_readable_size, progress_bar_init};
use comfy_table::{Cell, ContentArrangement, Table};
use indicatif::ProgressBar;
use rayon::prelude::*;
use std::fs;
use std::path::{Path, PathBuf, MAIN_SEPARATOR};

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

        for (_i, file) in files.iter().enumerate() {
            process_pb.tick();
            let file_path = path.join(&file);
            if args.name.is_some() {
                let metadata = match file_path.metadata() {
                    Ok(m) => m,
                    Err(e) => {
                        eprintln!("ls: cannot access '{}': {}", file_path.display(), e);
                        continue;
                    }
                };
                if metadata.is_dir() {
                    // 如果是目录，是否跟要搜索的名称匹配
                    if let Some(name) = &args.name {
                        if !file.contains(name) {
                            calculate_dir_size1(
                                file_path,
                                args.human_readable,
                                &process_pb,
                                args.parallel,
                                &name,
                                &mut entries,
                            );
                            continue;
                        }
                    }
                } else {
                    continue;
                }
            }
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
                path: match file_path.canonicalize() {
                    Ok(canonical_path) => {
                        let path_str = canonical_path.to_string_lossy().into_owned();
                        let path_str = path_str.strip_prefix(r"\\?\").unwrap_or(&path_str);
                        path_str.to_string()
                    }
                    Err(_e) => {
                        // eprintln!("获取绝对路径失败: {}", e);
                        file_path.to_string_lossy().into_owned()
                    }
                },
            });
        }

        process_pb.finish_and_clear();
        let mut sum_size = 0;
        for entry in &entries {
            sum_size += entry.size_raw; // 使用第4个字段的原始大小
        }
        if args.sort {
            entries.sort_by(|a, b| a.size_raw.cmp(&b.size_raw));
        }

        let mut table = Table::new();
        table
            .set_content_arrangement(ContentArrangement::Dynamic)
            .set_header(vec![
                Cell::new("类型").add_attribute(comfy_table::Attribute::Bold),
                Cell::new("权限").add_attribute(comfy_table::Attribute::Bold),
                Cell::new("大小").add_attribute(comfy_table::Attribute::Bold),
                Cell::new("路径").add_attribute(comfy_table::Attribute::Bold),
            ])
            .load_preset(comfy_table::presets::UTF8_FULL)
            .apply_modifier(comfy_table::modifiers::UTF8_ROUND_CORNERS);

        for entry in entries.iter() {
            let file_path = if args.full_path {
                &entry.path
            } else {
                entry
                    .path
                    .split(MAIN_SEPARATOR)
                    .last()
                    .unwrap_or(&entry.path)
            };
            table.add_row(vec![
                Cell::new(&entry.file_type.to_string())
                    .set_alignment(comfy_table::CellAlignment::Center),
                Cell::new(entry.permissions.replace('-', "")),
                Cell::new(&entry.size_display),
                Cell::new(file_path),
            ]);
        }

        println!("{}", table);
        println!("┌{:─^33}┐", "");
        println!(
            "│ 总数量: {:6} │ 总大小: {:10} ",
            entries.len(),
            human_readable_size(sum_size)
        );
        println!("└{:─^33}┘", "");
    } else {
        for file in files {
            println!("{}", file);
        }
    }
    scan_pb.finish_and_clear(); // 完成后清理进度条
}

// 需要重写一个函数，是实现传入一个目录，传入一个名称，返回这个目录下面的对应名称文件大小
fn calculate_dir_size1(
    file_path: PathBuf,
    human_readable: bool,
    pb: &ProgressBar,
    main_pb: bool,
    name: &str,
    entries: &mut Vec<FileEntry>,
) {
    let sub_path_str = file_path.display().to_string();
    let sub_path = Path::new(&sub_path_str);
    // 怎么进入到这个目录下面
    let sub_entries = match fs::read_dir(sub_path) {
        Ok(entries) => entries,
        Err(e) => {
            eprintln!("ls: cannot access '{}': {}", sub_path.display(), e);
            return;
        }
    };
    for entry in sub_entries.flatten() {
        let file_name = entry.file_name().to_string_lossy().to_string();
        let metadata = match entry.metadata() {
            Ok(m) => m,
            Err(e) => {
                eprintln!("ls: cannot access '{}': {}", sub_path.display(), e);
                continue;
            }
        };
        if metadata.is_dir() {
            let file_path = sub_path.join(&file_name);
            // 如果是目录，是否跟要搜索的名称匹配
            if !file_name.contains(name) {
                calculate_dir_size1(file_path, human_readable, pb, main_pb, &name, entries);
                continue; // 如果不匹配则跳过
            } else {
                let (raw, converted) = calculate_dir_size(&file_path, human_readable, pb, main_pb);
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
                    size_display: converted,
                    size_raw: raw,
                    path: match file_path.canonicalize() {
                        Ok(canonical_path) => {
                            let path_str = canonical_path.to_string_lossy().into_owned();
                            let path_str = path_str.strip_prefix(r"\\?\").unwrap_or(&path_str);
                            path_str.to_string()
                        }
                        Err(e) => {
                            eprintln!("获取绝对路径失败: {}", e);
                            "".to_string()
                        }
                    },
                });
            }
        } else {
            continue;
        }
    }
}
