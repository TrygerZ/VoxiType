//! Audio input device enumeration.

use cpal::traits::{DeviceTrait, HostTrait};
use serde::{Deserialize, Serialize};

use crate::error::{AppError, Result};

/// A selectable microphone device.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfo {
    pub id: String,
    pub name: String,
    pub is_default: bool,
}

/// List all available input devices, marking the system default.
pub fn list_input_devices() -> Result<Vec<DeviceInfo>> {
    let host = cpal::default_host();
    let default_name = host
        .default_input_device()
        .and_then(|d| d.name().ok());

    let devices = host
        .input_devices()
        .map_err(|e| AppError::audio(format!("Failed to enumerate input devices: {e}")))?;

    let mut out = Vec::new();
    out.push(DeviceInfo {
        id: "default".to_string(),
        name: "System Default".to_string(),
        is_default: true,
    });

    for device in devices {
        if let Ok(name) = device.name() {
            let is_default = default_name.as_deref() == Some(name.as_str());
            out.push(DeviceInfo {
                id: name.clone(),
                name,
                is_default,
            });
        }
    }

    Ok(out)
}

/// Resolve a device id ("default" or a device name) into a cpal device.
pub fn resolve_device(id: &str) -> Result<cpal::Device> {
    let host = cpal::default_host();
    if id == "default" {
        return host
            .default_input_device()
            .ok_or_else(|| AppError::audio_device_not_found("No default input device available"));
    }

    let devices = host
        .input_devices()
        .map_err(|e| AppError::audio(format!("Failed to enumerate input devices: {e}")))?;
    for device in devices {
        if let Ok(name) = device.name() {
            if name == id {
                return Ok(device);
            }
        }
    }
    Err(AppError::audio_device_not_found(format!(
        "Input device '{id}' not found"
    )))
}
