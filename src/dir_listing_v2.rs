use std::path::Path;
use std::sync::atomic::AtomicU64;
use crate::models::Cli;
use std::sync::atomic::Ordering;
use crate::utils;
pub fn list_directory_v2(path: &Path, args: &Cli) {
    println!("使用迭代方法{:?}",path);
    // 添加时间
    
    jwalk_size(path);
}

fn jwalk_size(path: &Path) -> std::io::Result<u64> {
    let total_size: u64 = jwalk::WalkDir::new(path)
    
        .parallelism(jwalk::Parallelism::RayonNewPool(16)) // 自定义线程数
        .into_iter()
        .filter_map(|entry| {
            entry.ok().and_then(|e| {
                if e.file_type().is_file() {
                    // println!("文件名称：{:?},文件大小：{:?}",e.file_name(),e.metadata().map(|m| m.len()).ok());
                    // Some(e.file_name().len() as u64)
                    //   e.metadata().map(|m| m.len()).ok()
                    Some(0)
                } else {
                    None
                }
            })
        })
        .sum();
    println!("{:?}",total_size);
    let size_str = utils::human_readable_size(total_size);
    println!("目录大小: {}", size_str);
    Ok(total_size)
}