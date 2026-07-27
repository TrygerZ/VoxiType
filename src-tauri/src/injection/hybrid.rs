//! Hybrid injector: clipboard paste with keystroke fallback.

use std::time::Instant;

use super::{clipboard, keystroke, InjectResult, InjectStrategy, TextInjector};
use crate::error::Result;

pub struct HybridInjector;

impl HybridInjector {
    pub fn new() -> Self {
        Self
    }
}

impl Default for HybridInjector {
    fn default() -> Self {
        Self
    }
}

impl TextInjector for HybridInjector {
    fn inject(&self, text: &str) -> Result<InjectResult> {
        if text.is_empty() {
            return Ok(InjectResult {
                success: true,
                strategy: InjectStrategy::Manual,
                chars_injected: 0,
                duration_ms: 0,
            });
        }

        // Try clipboard first.
        match self.inject_clipboard(text) {
            Ok(res) if res.success => Ok(res),
            _ => self.inject_keystroke(text),
        }
    }

    fn inject_clipboard(&self, text: &str) -> Result<InjectResult> {
        let started = Instant::now();
        
        // Save current clipboard content
        let prev = clipboard::read_text();
        
        clipboard::write_text(text)?;
        
        // Small delay to let target app consume the paste
        std::thread::sleep(std::time::Duration::from_millis(50));
        
        keystroke::paste()?;
        
        // Restore previous clipboard if we had one
        if let Some(prev_text) = prev {
            if let Err(e) = clipboard::write_text(&prev_text) {
                tracing::warn!("Failed to restore clipboard: {e}");
            }
        }
        
        Ok(InjectResult {
            success: true,
            strategy: InjectStrategy::Clipboard,
            chars_injected: text.chars().count() as u32,
            duration_ms: started.elapsed().as_millis() as u64,
        })
    }

    fn inject_keystroke(&self, text: &str) -> Result<InjectResult> {
        let started = Instant::now();
        let chars = keystroke::type_text(text)?;
        Ok(InjectResult {
            success: true,
            strategy: InjectStrategy::Keystroke,
            chars_injected: chars,
            duration_ms: started.elapsed().as_millis() as u64,
        })
    }
}
