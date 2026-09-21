use std::fs;
use std::path::Path;

fn main() {
    // 获取项目根目录
    let project_root = env!("CARGO_MANIFEST_DIR");
    // 获取编译脚本文件的输出目录路径
    let out_dir = std::env::var("OUT_DIR").unwrap();
    // 构造目标文件路径，通过向上回溯OUT_DIR的父级目录来定位
    let dest_dir_path = Path::new(&out_dir)
        .ancestors()
        .nth(3)
        .expect("Failed to get parent directory");

    const CONFIG_EXTS: &[&str] = &["toml", "json", "json5", "yml", "yaml", "ini", "ron"];

    let pkg_name = env!("CARGO_PKG_NAME");

    // 复制配置文件
    for ext in CONFIG_EXTS {
        let config_file = format!("{}/{}.{}", project_root, pkg_name, ext);
        let src_path = Path::new(&config_file);
        if src_path.exists() {
            let dest_file = format!("{}.{}", pkg_name, ext);
            let dest = dest_dir_path.join(&dest_file);
            fs::copy(src_path, &dest)
                .unwrap_or_else(|e| panic!("Failed to copy {} to {:?}: {}", config_file, dest, e));
        }
    }

    // 复制 log.toml
    let log_config = format!("{}/log.toml", project_root);
    let log_src = Path::new(&log_config);
    if log_src.exists() {
        let log_dest = dest_dir_path.join("log.toml");
        fs::copy(log_src, &log_dest)
            .unwrap_or_else(|e| panic!("Failed to copy {} to {:?}: {}", log_config, log_dest, e));
    }
}