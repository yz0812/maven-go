use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use walkdir::WalkDir;
use rayon::prelude::*;

// ===================== 数据结构 =====================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvalidArtifact {
    folder: String,
    base_name: String,
    reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanItem {
    folder: String,
    base_name: String,
}

#[derive(Debug, Serialize)]
pub struct CleanResult {
    deleted_count: usize,
    errors: Vec<String>,
}

// ===================== 常量配置 =====================

const MAX_JAR_SIZE: u64 = 1024; // 1KB
const BAD_POM_KEYWORDS: &[&str] = &[
    "<!DOCTYPE html>",
    "<title>Harbor</title>",
    "Login to Harbor",
];
const METADATA_FILES: &[&str] = &[
    "_remote.repositories",
    "_maven.repositories",
    "resolver-status.properties",
];

// ===================== Tauri Commands =====================

#[tauri::command]
fn get_maven_repo_path() -> Result<String, String> {
    // 辅助函数：从 settings.xml 解析 localRepository
    fn parse_local_repo(settings_path: &Path) -> Option<String> {
        println!("[DEBUG] 尝试读取配置文件: {}", settings_path.display());

        if !settings_path.exists() {
            println!("[DEBUG] 文件不存在");
            return None;
        }

        let content = match fs::read_to_string(settings_path) {
            Ok(c) => {
                println!("[DEBUG] 文件读取成功，长度: {} 字节", c.len());
                c
            }
            Err(e) => {
                println!("[DEBUG] 文件读取失败: {}", e);
                return None;
            }
        };

        let doc = match roxmltree::Document::parse(&content) {
            Ok(d) => {
                println!("[DEBUG] XML 解析成功");
                d
            }
            Err(e) => {
                println!("[DEBUG] XML 解析失败: {}", e);
                return None;
            }
        };

        for node in doc.descendants() {
            if node.has_tag_name("localRepository") {
                if let Some(repo_path) = node.text() {
                    let trimmed = repo_path.trim();
                    if !trimmed.is_empty() {
                        println!("[DEBUG] ✅ 找到 localRepository: {}", trimmed);
                        return Some(trimmed.to_string());
                    }
                }
            }
        }

        println!("[DEBUG] 未找到 <localRepository> 标签");
        None
    }

    // 辅助函数：通过 mvn -v 命令获取 Maven 安装路径
    fn get_maven_home_from_command() -> Option<String> {
        use std::process::Command;

        println!("[DEBUG] 尝试执行 mvn 命令");

        // Windows 下尝试 mvn.cmd 和 mvn.bat
        let maven_commands = if cfg!(target_os = "windows") {
            vec!["mvn.cmd", "mvn.bat", "mvn"]
        } else {
            vec!["mvn"]
        };

        for cmd in maven_commands {
            println!("[DEBUG] 尝试命令: {}", cmd);

            // 创建命令构建器
            let mut command = Command::new(cmd);
            command.arg("-v");

            // Windows 下隐藏命令行窗口
            #[cfg(target_os = "windows")]
            {
                use std::os::windows::process::CommandExt;
                const CREATE_NO_WINDOW: u32 = 0x08000000;
                command.creation_flags(CREATE_NO_WINDOW);
            }

            let output = match command.output() {
                Ok(o) => {
                    println!("[DEBUG] {} 执行成功", cmd);
                    o
                }
                Err(e) => {
                    println!("[DEBUG] {} 执行失败: {}", cmd, e);
                    continue;
                }
            };

            if !output.status.success() {
                println!("[DEBUG] {} 返回非零状态码", cmd);
                continue;
            }

            let stdout = String::from_utf8_lossy(&output.stdout);
            println!("[DEBUG] mvn -v 输出:\n{}", stdout);

            for line in stdout.lines() {
                if line.starts_with("Maven home:") {
                    let maven_home = line.strip_prefix("Maven home:")?.trim();
                    println!("[DEBUG] ✅ 从 mvn -v 解析到 Maven home: {}", maven_home);
                    return Some(maven_home.to_string());
                }
            }
        }

        println!("[DEBUG] 所有 mvn 命令尝试均失败");
        None
    }

    println!("\n========== 开始检测 Maven 仓库路径 ==========");

    // 1. 最高优先级：通过 mvn -v 命令获取的 Maven 全局配置
    println!("[步骤 1] 尝试通过 mvn -v 命令检测");
    if let Some(maven_home) = get_maven_home_from_command() {
        let global_settings = Path::new(&maven_home).join("conf").join("settings.xml");
        if let Some(repo) = parse_local_repo(&global_settings) {
            println!("========== ✅ 检测成功，返回路径: {} ==========\n", repo);
            return Ok(repo);
        }
    }

    // 2. 次优先级：环境变量指定的 Maven 全局配置
    println!("[步骤 2] 尝试读取环境变量 MAVEN_HOME / M2_HOME");
    let mut maven_home_candidates = vec![
        std::env::var("MAVEN_HOME").ok(),
        std::env::var("M2_HOME").ok(),
    ];

    for (i, maven_home) in maven_home_candidates.iter().enumerate() {
        if let Some(home) = maven_home {
            println!("[DEBUG] 环境变量 {} = {}", if i == 0 { "MAVEN_HOME" } else { "M2_HOME" }, home);
        } else {
            println!("[DEBUG] 环境变量 {} 未设置", if i == 0 { "MAVEN_HOME" } else { "M2_HOME" });
        }
    }

    // 3. 尝试从 PATH 环境变量推断 Maven 路径
    println!("[步骤 2.5] 尝试从 PATH 环境变量推断 Maven 路径");
    if let Ok(path_env) = std::env::var("PATH") {
        println!("[DEBUG] PATH 环境变量已设置");
        for path in path_env.split(';') {
            if path.to_lowercase().contains("maven") && path.to_lowercase().contains("bin") {
                println!("[DEBUG] 发现可能的 Maven bin 目录: {}", path);
                // Maven bin 路径的父目录就是 Maven home
                if let Some(parent) = Path::new(path).parent() {
                    let maven_home = parent.to_string_lossy().to_string();
                    println!("[DEBUG] 推断的 Maven home: {}", maven_home);
                    maven_home_candidates.push(Some(maven_home));
                }
            }
        }
    }

    for maven_home in maven_home_candidates.into_iter().flatten() {
        let global_settings = Path::new(&maven_home).join("conf").join("settings.xml");
        if let Some(repo) = parse_local_repo(&global_settings) {
            println!("========== ✅ 检测成功，返回路径: {} ==========\n", repo);
            return Ok(repo);
        }
    }

    // 3. 第三优先级：用户级别 settings.xml (~/.m2/settings.xml)
    println!("[步骤 3] 尝试读取用户级配置 ~/.m2/settings.xml");
    if let Some(home_dir) = dirs::home_dir() {
        println!("[DEBUG] 用户主目录: {}", home_dir.display());
        let user_settings = home_dir.join(".m2").join("settings.xml");
        if let Some(repo) = parse_local_repo(&user_settings) {
            println!("========== ✅ 检测成功，返回路径: {} ==========\n", repo);
            return Ok(repo);
        }
    }

    // 4. 兜底：返回默认路径 ~/.m2/repository
    println!("[步骤 4] 使用默认路径");
    let home_dir = dirs::home_dir().ok_or("无法获取用户主目录")?;
    let default_repo = home_dir.join(".m2").join("repository");
    let default_path = default_repo.to_string_lossy().to_string();
    println!("========== ⚠️ 使用默认路径: {} ==========\n", default_path);
    Ok(default_path)
}

#[tauri::command]
fn scan_invalid_artifacts(repo_path: String) -> Result<Vec<InvalidArtifact>, String> {
    let repo_path = Path::new(&repo_path);

    if !repo_path.exists() {
        return Err(format!("仓库路径不存在: {}", repo_path.display()));
    }

    if !repo_path.is_dir() {
        return Err(format!("路径不是目录: {}", repo_path.display()));
    }

    // 根据 CPU 核心数配置线程池 (IO 密集型,设为核心数 * 4)
    let cpu_count = num_cpus::get();
    let thread_count = cpu_count * 4;

    println!("[多线程扫描] CPU 核心数: {}, 线程池大小: {}", cpu_count, thread_count);

    // 配置 Rayon 全局线程池
    rayon::ThreadPoolBuilder::new()
        .num_threads(thread_count)
        .build_global()
        .ok(); // 忽略重复初始化错误

    // 第一阶段：收集所有待检查的文件路径
    let files_to_check: Vec<_> = WalkDir::new(repo_path)
        .into_iter()
        .filter_entry(|e| {
            // 跳过隐藏目录
            !e.file_name()
                .to_str()
                .map(|s| s.starts_with('.'))
                .unwrap_or(false)
        })
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_file())
        .filter(|e| {
            // 只处理 .jar 和 .pom 文件
            e.path()
                .extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| ext == "jar" || ext == "pom")
                .unwrap_or(false)
        })
        .map(|e| e.path().to_path_buf())
        .collect();

    println!("[多线程扫描] 发现 {} 个 JAR/POM 文件,开始并行检查...", files_to_check.len());

    // 第二阶段：并行检查所有文件
    let invalid_artifacts: Vec<InvalidArtifact> = files_to_check
        .par_iter() // 使用 Rayon 并行迭代器
        .filter_map(|path| {
            let file_name = path.file_name()?.to_str()?;
            let mut is_bad = false;
            let mut reason = String::new();

            // 检查损坏的 JAR
            if file_name.ends_with(".jar") {
                if let Ok(metadata) = fs::metadata(path) {
                    if metadata.len() < MAX_JAR_SIZE {
                        is_bad = true;
                        reason = format!("小于{}字节的JAR文件", MAX_JAR_SIZE);
                    }
                }
            }
            // 检查损坏的 POM
            else if file_name.ends_with(".pom") {
                if let Ok(content) = fs::read_to_string(path) {
                    let preview = &content.chars().take(1024).collect::<String>();
                    for keyword in BAD_POM_KEYWORDS {
                        if preview.contains(keyword) {
                            is_bad = true;
                            reason = "包含Harbor错误页面的POM文件".to_string();
                            break;
                        }
                    }
                }
            }

            if is_bad {
                let parent = path.parent()?;
                let base_name = file_name
                    .trim_end_matches(".jar")
                    .trim_end_matches(".pom")
                    .to_string();

                Some(InvalidArtifact {
                    folder: parent.to_string_lossy().to_string(),
                    base_name,
                    reason,
                })
            } else {
                None
            }
        })
        .collect();

    println!("[多线程扫描] 扫描完成,发现 {} 个损坏的构件", invalid_artifacts.len());

    Ok(invalid_artifacts)
}

#[tauri::command]
fn clean_artifacts(items: Vec<CleanItem>) -> Result<CleanResult, String> {
    let mut deleted_count = 0;
    let mut errors = Vec::new();

    for item in items {
        let folder = Path::new(&item.folder);

        if !folder.exists() {
            continue;
        }

        let entries = match fs::read_dir(folder) {
            Ok(entries) => entries,
            Err(e) => {
                errors.push(format!("无法读取目录 {}: {}", folder.display(), e));
                continue;
            }
        };

        for entry in entries.flatten() {
            let file_path = entry.path();
            let file_name = match file_path.file_name().and_then(|n| n.to_str()) {
                Some(name) => name,
                None => continue,
            };

            let should_delete = file_name.starts_with(&item.base_name)
                || METADATA_FILES.contains(&file_name);

            if should_delete {
                match fs::remove_file(&file_path) {
                    Ok(_) => deleted_count += 1,
                    Err(e) => {
                        errors.push(format!("删除失败 {}: {}", file_path.display(), e));
                    }
                }
            }
        }
    }

    Ok(CleanResult {
        deleted_count,
        errors,
    })
}

// ===================== 应用入口 =====================

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_fs::init())
        .invoke_handler(tauri::generate_handler![
            get_maven_repo_path,
            scan_invalid_artifacts,
            clean_artifacts
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
