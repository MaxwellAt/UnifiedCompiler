use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct Digest {
    pub sha256: String,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileNode {
    pub relative_path: String,
    pub digest: Digest,
    pub is_executable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceManifest {
    pub files: Vec<FileNode>,
    pub directories: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolchainInfo {
    pub name: String,
    pub version: String,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeCapabilities {
    pub os: String,
    pub arch: String,
    pub hostname: String,
    pub cpu_cores: usize,
    pub total_memory_mb: u64,
    pub toolchains: Vec<ToolchainInfo>,
    pub server_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum LanguagePreset {
    Rust,
    Python,
    Node,
    Jvm,
    Go,
    Cpp,
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionConfig {
    pub job_id: String,
    pub project_name: String,
    pub language: LanguagePreset,
    pub command: Vec<String>,
    pub env_vars: HashMap<String, String>,
    pub sync_artifacts: Vec<String>,
    pub pty_mode: bool,
    pub term_cols: u16,
    pub term_rows: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ExecutionPhase {
    Pending,
    SynchronizingWorkspace,
    HydratingDependencies,
    Compiling,
    Running,
    Finished,
    Failed,
}

/// Messages sent from Master/Client to Host/Worker
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClientMessage {
    Handshake {
        client_version: String,
        auth_token: Option<String>,
    },
    QueryMissingBlobs {
        digests: Vec<Digest>,
    },
    UploadBlobChunk {
        digest: Digest,
        data: Vec<u8>,
        is_compressed: bool,
    },
    StartExecution {
        config: ExecutionConfig,
        manifest: WorkspaceManifest,
    },
    StdinData {
        data: Vec<u8>,
    },
    ResizeTerminal {
        cols: u16,
        rows: u16,
    },
    ForwardSignal {
        signal: i32,
    },
    CancelExecution,
}

/// Messages sent from Host/Worker to Master/Client
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HostMessage {
    HandshakeAck {
        capabilities: NodeCapabilities,
    },
    MissingBlobs {
        digests: Vec<Digest>,
    },
    BlobsAck {
        stored_count: usize,
    },
    PhaseUpdate {
        phase: ExecutionPhase,
        message: String,
    },
    StdoutChunk {
        data: Vec<u8>,
    },
    StderrChunk {
        data: Vec<u8>,
    },
    ProcessTelemetry {
        cpu_percent: f32,
        memory_mb: u64,
    },
    ExecutionFinished {
        exit_code: i32,
        duration_ms: u64,
        error_message: Option<String>,
    },
    ArtifactSync {
        relative_path: String,
        data: Vec<u8>,
    },
    Error {
        message: String,
    },
}
