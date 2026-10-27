use std::fs;
use std::io::Write;
use std::path::Path;

/// 原子写入：先写临时文件，成功后再改名覆盖目标。
pub fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let tmp = path.with_extension("tmp");
    // 1. 把 bytes 写进 tmp
    {
        let mut file =
            fs::File::create(&tmp).map_err(|e| format!("创建文件失败 {:?}: {}", tmp, e))?;
        file.write_all(bytes)
            .map_err(|e| format!("写入文件失败 {:?}: {}", tmp, e))?;
        file.sync_all()
            .map_err(|e| format!("同步文件失败 {:?}: {}", tmp, e))?;
    }
    // 2. 把 tmp 改名成 path
    fs::rename(&tmp, path).map_err(|e| format!("重命名文件失败 {:?} → {:?}: {}", tmp, path, e))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch_dir() -> std::path::PathBuf {
        // 用进程 ID 做名字，避免并行测试互相踩
        let dir = std::env::temp_dir().join(format!("moe-wp-test-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn write_atomic_writes_bytes() {
        let dir = scratch_dir();
        let target = dir.join("current.jpg");

        write_atomic(&target, b"hello").unwrap();

        assert_eq!(fs::read(&target).unwrap(), b"hello");
        assert!(!dir.join("current.tmp").exists()); // 临时文件必须被"消费掉"

        fs::remove_dir_all(&dir).unwrap(); // 打扫干净
    }
}
