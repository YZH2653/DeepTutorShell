use serde::{Deserialize, Serialize};
use std::path::PathBuf;

// 默认路径来自任务书部署事实
pub const DEFAULT_DEEPTUTOR_EXE: &str = r"E:\DeepTutor\.venv\Scripts\deeptutor.exe";
pub const DEFAULT_WORKSPACE: &str = r"E:\DeepTutor\dt-home";
pub const DEFAULT_FRONTEND_PORT: u16 = 3782;
pub const DEFAULT_BACKEND_PORT: u16 = 8001;
// BGE-M3 本地嵌入服务（知识库检索依赖；v1.1.0.0 起与 DeepTutor 联动拉起）
pub const DEFAULT_EMBED_PYTHON: &str = r"E:\DeepTutor\.venv\Scripts\python.exe";
pub const DEFAULT_EMBED_SCRIPT: &str = r"E:\BGE-M3\serve_bge_m3.py";
pub const DEFAULT_EMBED_PORT: u16 = 6066;

// 嵌入服务子配置（python + 脚本 + 端口；模型加载慢，就绪超时单独放宽）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct EmbeddingServerConfig {
    pub python: String,
    pub script: String,
    pub port: u16,
    pub ready_timeout_secs: u64,
}

impl Default for EmbeddingServerConfig {
    fn default() -> Self {
        Self {
            python: DEFAULT_EMBED_PYTHON.to_string(),
            script: DEFAULT_EMBED_SCRIPT.to_string(),
            port: DEFAULT_EMBED_PORT,
            ready_timeout_secs: 150,
        }
    }
}

// 壳配置（exe 旁 config.json，缺文件用默认值）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    pub deeptutor_exe: String,
    pub workspace: String,
    pub frontend_port: u16,
    pub backend_port: u16,
    pub embedding_server: EmbeddingServerConfig,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            deeptutor_exe: DEFAULT_DEEPTUTOR_EXE.to_string(),
            workspace: DEFAULT_WORKSPACE.to_string(),
            frontend_port: DEFAULT_FRONTEND_PORT,
            backend_port: DEFAULT_BACKEND_PORT,
            embedding_server: EmbeddingServerConfig::default(),
        }
    }
}

impl AppConfig {
    // 读取 exe 旁 config.json；缺失或损坏时用默认值
    pub fn load() -> Self {
        let path = exe_dir().join("config.json");
        std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }
}

// 当前 exe 所在目录
pub fn exe_dir() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(PathBuf::from))
        .unwrap_or_else(|| PathBuf::from("."))
}

#[cfg(test)]
mod tests {
    use super::*;

    // 默认值与任务书一致
    #[test]
    fn defaults_match_spec() {
        let c = AppConfig::default();
        assert_eq!(c.deeptutor_exe, r"E:\DeepTutor\.venv\Scripts\deeptutor.exe");
        assert_eq!(c.workspace, r"E:\DeepTutor\dt-home");
        assert_eq!(c.frontend_port, 3782);
        assert_eq!(c.backend_port, 8001);
    }

    // v1.1.0.0：嵌入服务默认值（BGE-M3）
    #[test]
    fn embedding_defaults() {
        let e = AppConfig::default().embedding_server;
        assert_eq!(e.python, r"E:\DeepTutor\.venv\Scripts\python.exe");
        assert_eq!(e.script, r"E:\BGE-M3\serve_bge_m3.py");
        assert_eq!(e.port, 6066);
        assert_eq!(e.ready_timeout_secs, 150);
    }

    // 嵌入配置部分字段覆盖、其余回默认
    #[test]
    fn embedding_partial_override() {
        let c: AppConfig =
            serde_json::from_str(r#"{"embedding_server":{"port":7000}}"#).unwrap();
        assert_eq!(c.embedding_server.port, 7000);
        assert_eq!(c.embedding_server.ready_timeout_secs, 150);
        assert_eq!(c.embedding_server.script, DEFAULT_EMBED_SCRIPT);
    }

    #[test]
    fn full_json_parses() {
        let c: AppConfig = serde_json::from_str(
            r#"{"deeptutor_exe":"C:\\x\\deeptutor.exe","workspace":"C:\\w","frontend_port":4000,"backend_port":5000}"#,
        )
        .unwrap();
        assert_eq!(c.deeptutor_exe, r"C:\x\deeptutor.exe");
        assert_eq!(c.workspace, r"C:\w");
        assert_eq!(c.frontend_port, 4000);
        assert_eq!(c.backend_port, 5000);
    }

    // 部分字段缺失时其余用默认值
    #[test]
    fn partial_json_fills_defaults() {
        let c: AppConfig = serde_json::from_str(r#"{"frontend_port":4000}"#).unwrap();
        assert_eq!(c.frontend_port, 4000);
        assert_eq!(c.backend_port, DEFAULT_BACKEND_PORT);
        assert_eq!(c.workspace, DEFAULT_WORKSPACE);
        assert_eq!(c.deeptutor_exe, DEFAULT_DEEPTUTOR_EXE);
    }

    // 坏 JSON 回退默认值
    #[test]
    fn malformed_json_falls_back_to_default() {
        let c = serde_json::from_str::<AppConfig>("not json")
            .ok()
            .unwrap_or_default();
        assert_eq!(c.frontend_port, DEFAULT_FRONTEND_PORT);
        assert_eq!(c.backend_port, DEFAULT_BACKEND_PORT);
    }
}
