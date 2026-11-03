use std::fmt;
use std::io;
use std::time::Duration;

#[derive(Debug)]
pub enum ProbeError {
    BinaryNotFound {
        binary: String,
        source: io::Error,
    },
    Timeout {
        duration: Duration,
    },
    ExecutionFailed {
        exit_code: Option<i32>,
        stderr: String,
    },
    Deserialization(serde_json::Error),
    Io(io::Error),
}

impl fmt::Display for ProbeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BinaryNotFound { binary, source } => {
                write!(f, "ffprobe binary '{binary}' not found or cannot execute: {source}")
            }
            Self::Timeout { duration } => {
                write!(f, "ffprobe execution timed out after {duration:?}")
            }
            Self::ExecutionFailed { exit_code, stderr } => {
                write!(
                    f,
                    "ffprobe failed with exit code {:?}: {}",
                    exit_code,
                    stderr.trim()
                )
            }
            Self::Deserialization(err) => {
                write!(f, "failed to deserialize ffprobe JSON output: {err}")
            }
            Self::Io(err) => write!(f, "I/O error during ffprobe execution: {err}"),
        }
    }
}

impl std::error::Error for ProbeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::BinaryNotFound { source, .. } => Some(source),
            Self::Deserialization(err) => Some(err),
            Self::Io(err) => Some(err),
            Self::Timeout { .. } | Self::ExecutionFailed { .. } => None,
        }
    }
}

impl From<io::Error> for ProbeError {
    fn from(err: io::Error) -> Self {
        Self::Io(err)
    }
}

impl From<serde_json::Error> for ProbeError {
    fn from(err: serde_json::Error) -> Self {
        Self::Deserialization(err)
    }
}
