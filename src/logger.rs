//! OCR 请求日志系统
//!
//! 记录每次 OCR/结构识别请求的详细信息（JSON 行格式），
//! 支持配置日志存储路径和自动清理过期日志。
//!
//! 环境变量:
//! - `LOG_DIR` — 日志存储目录，默认 `./logs`
//! - `LOG_RETENTION_DAYS` — 日志保留天数，默认 30

use serde::Serialize;
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// 单条 OCR 请求日志
#[derive(Debug, Clone, Serialize)]
pub struct OcrLogEntry {
    pub timestamp: String,
    pub request_type: String,   // "ocr" | "structure"
    pub image_source: String,   // "upload" | "url"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_url: Option<String>,
    pub image_size_bytes: u64,
    pub result_count: usize,
    pub total_text_length: usize,
    pub duration_ms: u64,
    pub status: String,         // "success" | "error"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
}

/// OCR 日志记录器
#[derive(Clone)]
pub struct OcrLogger {
    log_dir: PathBuf,
    retention_days: u64,
}

impl OcrLogger {
    /// 从环境变量创建日志记录器
    pub fn from_env() -> Self {
        let log_dir = PathBuf::from(
            std::env::var("LOG_DIR").unwrap_or_else(|_| "./logs".to_string()),
        );
        let retention_days: u64 = std::env::var("LOG_RETENTION_DAYS")
            .unwrap_or_else(|_| "30".to_string())
            .parse()
            .unwrap_or(30);

        // 确保日志目录存在
        if let Err(e) = fs::create_dir_all(&log_dir) {
            tracing::warn!("无法创建日志目录 {}: {e}", log_dir.display());
        }

        tracing::info!(
            "OCR 日志系统已启用: 目录={}, 保留={}天",
            log_dir.display(),
            retention_days
        );

        Self {
            log_dir,
            retention_days,
        }
    }

    /// 写入一条日志（JSON 行）
    pub fn write(&self, entry: &OcrLogEntry) {
        let today = today_str();
        let log_file = self.log_dir.join(format!("ocr-{today}.jsonl"));

        match File::options().create(true).append(true).open(&log_file) {
            Ok(f) => {
                let mut writer = BufWriter::new(f);
                if let Err(e) = serde_json::to_writer(&mut writer, entry) {
                    tracing::error!("序列化日志失败: {e}");
                    return;
                }
                if let Err(e) = writer.write_all(b"\n") {
                    tracing::error!("写入日志换行失败: {e}");
                    return;
                }
                if let Err(e) = writer.flush() {
                    tracing::error!("刷新日志缓冲区失败: {e}");
                }
            }
            Err(e) => {
                tracing::error!("打开日志文件失败 {}: {e}", log_file.display());
            }
        }
    }

    /// 清理过期日志文件
    pub fn cleanup(&self) {
        let cutoff = SystemTime::now()
            .checked_sub(Duration::from_secs(self.retention_days * 86400));

        let Some(cutoff) = cutoff else {
            return;
        };

        let cutoff_secs = match cutoff.duration_since(UNIX_EPOCH) {
            Ok(d) => d.as_secs(),
            Err(_) => return,
        };

        match fs::read_dir(&self.log_dir) {
            Ok(entries) => {
                let mut deleted = 0u32;
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().map_or(true, |e| e != "jsonl") {
                        continue;
                    }

                    // 检查文件修改时间
                    if let Ok(meta) = entry.metadata() {
                        if let Ok(modified) = meta.modified() {
                            if let Ok(modified_dur) =
                                modified.duration_since(UNIX_EPOCH)
                            {
                                if modified_dur.as_secs() < cutoff_secs {
                                    if fs::remove_file(&path).is_ok() {
                                        deleted += 1;
                                        tracing::info!(
                                            "已删除过期日志: {}",
                                            path.display()
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
                if deleted > 0 {
                    tracing::info!("日志清理完成: 删除 {deleted} 个过期文件");
                }
            }
            Err(e) => {
                tracing::warn!("读取日志目录失败: {e}");
            }
        }
    }
}

/// 获取今天的日期字符串 YYYY-MM-DD
fn today_str() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    // 使用简单的日期计算（避免引入 chrono 依赖）
    let days_since_epoch = secs / 86400;
    let (y, m, d) = civil_from_days(days_since_epoch as i64);
    format!("{y:04}-{m:02}-{d:02}")
}

/// 从 Unix epoch 天数转换为公历日期（简化版）
pub fn civil_from_days(days: i64) -> (i64, u32, u32) {
    // 基于 Howard Hinnant 的算法
    let z = days + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y, m as u32, d as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_today_str() {
        let s = today_str();
        assert_eq!(s.len(), 10);
        assert!(s.chars().nth(4) == Some('-'));
        assert!(s.chars().nth(7) == Some('-'));
    }

    #[test]
    fn test_civil_from_days() {
        // 2026-06-23 = 20627 days since Unix epoch
        let (y, m, d) = civil_from_days(20627);
        assert_eq!(y, 2026);
        assert_eq!(m, 6);
        assert_eq!(d, 23);
    }
}