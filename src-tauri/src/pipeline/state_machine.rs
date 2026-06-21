//! Application state machine.
//!
//! Models the recording lifecycle: Idle -> Recording -> Processing -> Idle,
//! with an Error state reachable from anywhere and recoverable into Recording.

use std::time::Instant;

use serde::Serialize;

use crate::error::{AppError, ErrorCode};

/// Recording timeout in seconds.
pub const RECORDING_TIMEOUT_SECS: u64 = 60;

/// Runtime state. Heavy payloads (audio) are kept out of the serialized form.
#[derive(Debug)]
pub enum AppState {
    Idle,
    Recording { start_time: Instant },
    Processing { start_time: Instant },
    Error { message: String, code: ErrorCode },
}

/// Serializable snapshot of the state, emitted to the frontend.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum AppStateTag {
    Idle,
    Recording,
    Processing,
    Error,
}

impl AppState {
    pub fn tag(&self) -> AppStateTag {
        match self {
            AppState::Idle => AppStateTag::Idle,
            AppState::Recording { .. } => AppStateTag::Recording,
            AppState::Processing { .. } => AppStateTag::Processing,
            AppState::Error { .. } => AppStateTag::Error,
        }
    }
}

/// Events that drive transitions.
#[derive(Debug, Clone)]
pub enum StateEvent {
    StartRecording,
    StopRecording,
    CancelRecording,
    ProcessingComplete,
    Timeout,
    Error { message: String, code: ErrorCode },
}

impl AppState {
    /// Apply an event, returning the next state or the original state + error.
    pub fn transition(self, event: StateEvent) -> Result<AppState, (AppState, AppError)> {
        match (&self, &event) {
            (AppState::Idle, StateEvent::StartRecording)
            | (AppState::Error { .. }, StateEvent::StartRecording) => Ok(AppState::Recording {
                start_time: Instant::now(),
            }),

            (AppState::Recording { .. }, StateEvent::StopRecording) => Ok(AppState::Processing {
                start_time: Instant::now(),
            }),

            (AppState::Recording { .. }, StateEvent::CancelRecording) => Ok(AppState::Idle),

            (AppState::Recording { start_time }, StateEvent::Timeout) => {
                if start_time.elapsed().as_secs() >= RECORDING_TIMEOUT_SECS {
                    Ok(AppState::Error {
                        message: "Recording timeout reached".into(),
                        code: ErrorCode::RecordingTimeout,
                    })
                } else {
                    Err((self, AppError::invalid_transition("Timeout before limit")))
                }
            }

            (AppState::Processing { .. }, StateEvent::ProcessingComplete) => Ok(AppState::Idle),

            (_, StateEvent::Error { message, code }) => Ok(AppState::Error {
                message: message.clone(),
                code: *code,
            }),

            _ => Err((self, AppError::invalid_transition("Invalid state transition"))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ptt_happy_path() {
        let s = AppState::Idle;
        let s = s.transition(StateEvent::StartRecording).unwrap();
        assert_eq!(s.tag(), AppStateTag::Recording);
        let s = s.transition(StateEvent::StopRecording).unwrap();
        assert_eq!(s.tag(), AppStateTag::Processing);
        let s = s.transition(StateEvent::ProcessingComplete).unwrap();
        assert_eq!(s.tag(), AppStateTag::Idle);
    }

    #[test]
    fn cancel_returns_to_idle() {
        let s = AppState::Idle
            .transition(StateEvent::StartRecording)
            .unwrap();
        let s = s.transition(StateEvent::CancelRecording).unwrap();
        assert_eq!(s.tag(), AppStateTag::Idle);
    }

    #[test]
    fn error_then_recover() {
        let s = AppState::Idle.transition(StateEvent::Error {
            message: "boom".into(),
            code: ErrorCode::Internal,
        });
        let s = s.unwrap();
        assert_eq!(s.tag(), AppStateTag::Error);
        let s = s.transition(StateEvent::StartRecording).unwrap();
        assert_eq!(s.tag(), AppStateTag::Recording);
    }

    #[test]
    fn invalid_transition_rejected() {
        let s = AppState::Idle;
        assert!(s.transition(StateEvent::StopRecording).is_err());
    }
}
