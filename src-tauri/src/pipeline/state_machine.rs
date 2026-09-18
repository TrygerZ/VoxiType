//! Application state machine.
//!
//! Models the recording lifecycle: Idle -> Recording -> Processing -> Idle,
//! with an Error state reachable from anywhere and recoverable into Recording.

use std::time::Instant;

use serde::Serialize;

use crate::error::{AppError, ErrorCode};

/// Runtime state. Heavy payloads (audio) are kept out of the serialized form.
#[derive(Debug)]
pub enum AppState {
    Idle,
    Recording {
        start_time: Instant,
        active_app: Option<String>,
    },
    Processing {
        start_time: Instant,
        active_app: Option<String>,
    },
    Error {
        message: String,
        code: ErrorCode,
    },
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
    StartRecording { active_app: Option<String> },
    StopRecording,
    CancelRecording,
    ProcessingComplete,
    Error { message: String, code: ErrorCode },
}

impl AppState {
    /// Apply an event, returning the next state or the original state + error.
    pub fn transition(self, event: StateEvent) -> Result<AppState, (AppState, AppError)> {
        match (&self, &event) {
            (AppState::Idle, StateEvent::StartRecording { active_app })
            | (AppState::Error { .. }, StateEvent::StartRecording { active_app }) => {
                Ok(AppState::Recording {
                    start_time: Instant::now(),
                    active_app: active_app.clone(),
                })
            }

            (AppState::Recording { active_app, .. }, StateEvent::StopRecording) => {
                Ok(AppState::Processing {
                    start_time: Instant::now(),
                    active_app: active_app.clone(),
                })
            }

            (AppState::Recording { .. }, StateEvent::CancelRecording) => Ok(AppState::Idle),

            (AppState::Processing { .. }, StateEvent::ProcessingComplete) => Ok(AppState::Idle),

            (AppState::Idle, StateEvent::Error { message, code })
            | (AppState::Recording { .. }, StateEvent::Error { message, code })
            | (AppState::Processing { .. }, StateEvent::Error { message, code }) => {
                Ok(AppState::Error {
                    message: message.clone(),
                    code: *code,
                })
            }

            _ => {
                tracing::warn!(
                    from_state = ?self.tag(),
                    event = ?event,
                    "Invalid state transition rejected"
                );
                Err((
                    self,
                    AppError::invalid_transition("Invalid state transition"),
                ))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ptt_happy_path() {
        let s = AppState::Idle;
        let s = s
            .transition(StateEvent::StartRecording { active_app: None })
            .unwrap();
        assert_eq!(s.tag(), AppStateTag::Recording);
        let s = s.transition(StateEvent::StopRecording).unwrap();
        assert_eq!(s.tag(), AppStateTag::Processing);
        let s = s.transition(StateEvent::ProcessingComplete).unwrap();
        assert_eq!(s.tag(), AppStateTag::Idle);
    }

    #[test]
    fn cancel_returns_to_idle() {
        let s = AppState::Idle
            .transition(StateEvent::StartRecording { active_app: None })
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
        let s = s
            .transition(StateEvent::StartRecording { active_app: None })
            .unwrap();
        assert_eq!(s.tag(), AppStateTag::Recording);
    }

    #[test]
    fn invalid_transition_rejected() {
        let s = AppState::Idle;
        assert!(s.transition(StateEvent::StopRecording).is_err());
    }

    #[test]
    fn error_from_recording_and_processing_accepted() {
        let rec = AppState::Recording {
            start_time: Instant::now(),
            active_app: None,
        };
        let err_rec = rec.transition(StateEvent::Error {
            message: "rec fail".into(),
            code: ErrorCode::AudioDeviceError,
        });
        assert!(err_rec.is_ok());
        assert_eq!(err_rec.unwrap().tag(), AppStateTag::Error);

        let proc = AppState::Processing {
            start_time: Instant::now(),
            active_app: None,
        };
        let err_proc = proc.transition(StateEvent::Error {
            message: "proc fail".into(),
            code: ErrorCode::NetworkError,
        });
        assert!(err_proc.is_ok());
        assert_eq!(err_proc.unwrap().tag(), AppStateTag::Error);
    }

    #[test]
    fn error_from_error_state_is_rejected() {
        let err_state = AppState::Error {
            message: "first error".into(),
            code: ErrorCode::Internal,
        };
        let res = err_state.transition(StateEvent::Error {
            message: "second error".into(),
            code: ErrorCode::AudioDeviceError,
        });
        assert!(res.is_err());
        let (orig, err) = res.unwrap_err();
        assert_eq!(orig.tag(), AppStateTag::Error);
        assert_eq!(err.code, ErrorCode::InvalidTransition);
    }
}
