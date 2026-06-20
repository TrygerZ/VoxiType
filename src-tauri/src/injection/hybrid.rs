//! Hybrid injector: clipboard paste with keystroke fallback.

use std::thread;
use std::time::{Duration, Instant};

use super::{clipboard, keystroke, InjectResult, InjectStrategy, TextInjector};
use crate::error::Result;

/// Injection behaviour flags (from settings).
#[derive(Debug, Clone)]
pub struct InjectionOptions {
    pub auto_paste: bool,
    pub fallback_to_keystroke: bool,
    pub restore_clipboard: bool,
}

impl Default for InjectionOptions {
    fn default() -> Self {
        Self {
            auto_paste: true,
            fallback_to_keystroke: true,
            restore_clipboard: true,
        }
    }
}

pub struct HybridInjector {
    options: InjectionOptions,
}

impl HybridInjector {
    pub fn new(options: InjectionOptions) -> Self {
        Self { options }
    }
}

impl Default for HybridInjector {
    fn default() -> Self {
        Self::new(InjectionOptions::default())
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
            _ if self.options.fallback_to_keystroke => self.inject_keystroke(text),
            other => other,
        }
    }

    fn inject_clipboard(&self, text: &str) -> Result<InjectResult> {
        let started = Instant::now();
        let original = if self.options.restore_clipboard {
            clipboard::read_text()
        } else {
            None
        };

        clipboard::write_text(text)?;

        if self.options.auto_paste {
            keystroke::paste()?;
            // Give the target app time to consume the paste before restoring.
            thread::sleep(Duration::from_millis(120));
        }

        if self.options.restore_clipboard {
            if let Some(orig) = original {
                let _ = clipboard::write_text(&orig);
            }
        }

        Ok(InjectResult {
            success: true,
            strategy: if self.options.auto_paste {
                InjectStrategy::Clipboard
            } else {
                InjectStrategy::Manual
            },
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
