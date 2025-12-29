use super::models::{Cli, FileEntry};
use super::utils::{human_readable_size, progress_bar_init};
use comfy_table::{Cell, ContentArrangement, Table};
use indicatif::ProgressBar;
use rayon::prelude::*;
use std::fs;
use std::path::{Path, PathBuf, MAIN_SEPARATOR};
use std::sync::Arc;

pub fn calculate_dir_size(
    path: &Path,
    human_readable: bool,
    main_pb: &ProgressBar,
    parallel: bool,
) -> (u64, String) {
    // ✅ 设置当前计算的路径
    main_pb.set_message(format!("计算 {}...", path.display()));
    // 关键：用 Arc 包装，实现线程安全共享
    let pb_arc = Arc::new(main_pb.clone());

    let total = if parallel {
        // inner_calculate_parallel(path, &pb_arc, 0)
        inner_calculate_dynamic(path, &pb_arc, 0)
    } else {
        inner_calculate_serial(path, &pb_arc)
    };

    let converted = if human_readable {
        human_readable_size(total)
    } else {
        total.to_string()
    };
    (total, converted)
}
// 动态并行：根据目录复杂度决定是否并行
fn inner_calculate_dynamic(path: &Path, pb: &Arc<ProgressBar>, depth: usize) -> u64 {
    if depth > 0 && depth <= 2 {
        // 只显示前2层，避免消息刷新太频繁
        pb.set_message(format!("计算 {}...", path.display()));
    }
    match fs::read_dir(path) {
        Ok(entries) => {
            let entries_vec: Vec<_> = entries.collect();
            //根据深度决定tick频率
            let tick_freq = if depth == 0 {
                50
            } else if depth < 3 {
                100
            } else {
                200
            };
            // 收集条目并统计信息
            let items: Vec<_> = entries_vec
                .into_iter()
                .enumerate()
                .filter_map(|(i, e)| {
                    // 批量tick
                    if i % tick_freq == 0 {
                        pb.tick();
                    }

                    let entry = e.ok()?;
                    let metadata = entry.metadata().ok()?;
                    Some((entry.path(), metadata))
                })
                .collect();

            // 动态决策：是否使用并行
            let use_parallel = should_use_parallel(&items, depth);

            if use_parallel {
                // 并行处理
                items
                    .into_par_iter()
                    .map(|(item_path, metadata)| {
                        if metadata.is_dir() {
                            inner_calculate_dynamic(&item_path, pb, depth + 1)
                        } else {
                            metadata.len()
                        }
                    })
                    .sum()
            } else {
                // 串行处理
                let mut total = 0;
                for (item_path, metadata) in items {
                    if metadata.is_dir() {
                        total += inner_calculate_dynamic(&item_path, pb, depth + 1);
                    } else {
                        total += metadata.len();
                    }
                }
                total
            }
        }
        Err(e) => {
            eprintln!("无法读取目录 {}: {}", path.display(), e);
            0
        }
    }
}
// 智能决策：是否使用并行
fn should_use_parallel(items: &[(PathBuf, std::fs::Metadata)], depth: usize) -> bool {
    // 如果深度太大，直接返回false
    if depth > 10 {
        return false;
    }

    // 统计子目录数量
    let dir_count = items.iter().filter(|(_, m)| m.is_dir()).count();
    // 策略1：根据子目录数量决定
    //子目录越多，越应该并行
    if dir_count > 8 {
        return true;
    }

    // 策略2：根据总项数决定
    // 项数越多，越应该并行
    if items.len() > 100 {
        return true;
    }

    // 策略3：根据深度调整
    // 深度越大，越应该谨慎并行
    if depth > 5 {
        return dir_count > 4; // 只有子目录多才并行
    }

    // 策略4：混合模式
    // 浅层大胆并行，深层保守
    depth < 3 || (depth < 6 && dir_count > 2)
}

// 串行版本：用于深度过大或小目录
fn inner_calculate_serial(path: &Path, pb: &Arc<ProgressBar>) -> u64 {
    pb.set_message(format!("计算 {}...", path.display()));
    let mut total = 0;
    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            pb.tick();
            if let Ok(metadata) = entry.metadata() {
                if metadata.is_dir() {
                    total += inner_calculate_serial(&entry.path(), pb);
                } else {
                    total += metadata.len();
                }
            }
        }
    }
    total
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
        let pb_arc = Arc::new(&process_pb);
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
                            // 使用并行版本
                            calculate_dir_size_parallel(
                                file_path,
                                args.human_readable,
                                Arc::clone(&pb_arc), // 克隆 Arc
                                name,
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
                    Ok(canonical_path) => get_canonical_path(&canonical_path),
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

// 搜索文件
fn calculate_dir_size_parallel(
    file_path: PathBuf,
    human_readable: bool,
    pb: Arc<&ProgressBar>, // 改为 Arc
    name: &str,
    entries: &mut Vec<FileEntry>,
) {
    let sub_entries = match fs::read_dir(&file_path) {
        Ok(entries) => entries,
        Err(e) => {
            eprintln!("ls: cannot access '{}': {}", file_path.display(), e);
            return;
        }
    };

    // 收集所有需要处理的目录
    let dirs_to_process: Vec<_> = sub_entries
        .filter_map(|e| e.ok())
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            if name.starts_with('.') {
                return None;
            }
            let metadata = e.metadata().ok()?;
            if !metadata.is_dir() {
                return None;
            }
            Some((e.path(), name))
        })
        .collect();

    // 并行处理每个子目录
    let results: Vec<Vec<FileEntry>> = dirs_to_process
        .into_par_iter()
        .map(|(sub_path, sub_name)| {
            pb.tick();
            let mut local_entries = Vec::new();

            if sub_name.contains(name) {
                // 匹配：计算大小
                let (raw, converted) = calculate_dir_size(&sub_path, human_readable, &pb, true);
                local_entries.push(FileEntry {
                    file_type: 'd',
                    permissions: "rwx".to_string(),
                    size_display: converted,
                    size_raw: raw,
                    path: get_canonical_path(&sub_path),
                });
            } else {
                calculate_dir_size_parallel(
                    sub_path,
                    human_readable,
                    Arc::clone(&pb),
                    name,
                    &mut local_entries,
                );
            }
            local_entries
        })
        .collect();

    // 收集所有结果到主entries
    for result in results {
        entries.extend(result);
    }
}

fn get_canonical_path(path: &Path) -> String {
    match path.canonicalize() {
        Ok(canonical) => {
            let s = canonical.to_string_lossy().into_owned();
            s.strip_prefix(r"\\?\").unwrap_or(&s).to_string()
        }
        Err(_) => path.to_string_lossy().into_owned(),
    }
}
