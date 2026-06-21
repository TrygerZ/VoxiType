//! Opt-in anonymous usage statistics, aggregated per day.
//!
//! Nothing here is transmitted anywhere — it is local-only and recorded only
//! when the user enables the `telemetry` setting. It powers a simple in-app
//! "your usage" view and stays on the user's machine.

use serde::{Deserialize, Serialize};

use super::db::Database;
use crate::error::Result;

/// One day's aggregated counters.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DailyStats {
    pub date: String,
    pub transcription_count: i64,
    pub total_words: i64,
    pub total_duration_ms: i64,
    pub stt_local_count: i64,
    pub stt_cloud_count: i64,
    pub llm_local_count: i64,
    pub llm_cloud_count: i64,
    pub error_count: i64,
}

/// Engine origin used to bucket counts.
#[derive(Debug, Clone, Copy)]
pub enum EngineKind {
    Local,
    Cloud,
}

pub struct StatsRepository<'a> {
    db: &'a Database,
}

impl<'a> StatsRepository<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    /// Record one successful transcription against today's row (UTC date).
    pub fn record_transcription(
        &self,
        words: i64,
        duration_ms: i64,
        stt: EngineKind,
        llm: EngineKind,
    ) -> Result<()> {
        let (stt_local, stt_cloud) = match stt {
            EngineKind::Local => (1, 0),
            EngineKind::Cloud => (0, 1),
        };
        let (llm_local, llm_cloud) = match llm {
            EngineKind::Local => (1, 0),
            EngineKind::Cloud => (0, 1),
        };
        self.db.with_conn(|c| {
            c.execute(
                "INSERT INTO usage_stats
                   (date, transcription_count, total_words, total_duration_ms,
                    stt_local_count, stt_cloud_count, llm_local_count, llm_cloud_count)
                 VALUES (date('now'), 1, ?1, ?2, ?3, ?4, ?5, ?6)
                 ON CONFLICT(date) DO UPDATE SET
                   transcription_count = transcription_count + 1,
                   total_words = total_words + ?1,
                   total_duration_ms = total_duration_ms + ?2,
                   stt_local_count = stt_local_count + ?3,
                   stt_cloud_count = stt_cloud_count + ?4,
                   llm_local_count = llm_local_count + ?5,
                   llm_cloud_count = llm_cloud_count + ?6",
                rusqlite::params![
                    words,
                    duration_ms,
                    stt_local,
                    stt_cloud,
                    llm_local,
                    llm_cloud
                ],
            )?;
            Ok(())
        })
    }

    /// Record one error against today's row.
    pub fn record_error(&self) -> Result<()> {
        self.db.with_conn(|c| {
            c.execute(
                "INSERT INTO usage_stats (date, error_count)
                 VALUES (date('now'), 1)
                 ON CONFLICT(date) DO UPDATE SET error_count = error_count + 1",
                [],
            )?;
            Ok(())
        })
    }

    /// Return the most recent `days` of stats, newest first.
    pub fn recent(&self, days: u32) -> Result<Vec<DailyStats>> {
        self.db.with_conn(|c| {
            let mut stmt = c.prepare(
                "SELECT date, transcription_count, total_words, total_duration_ms,
                        stt_local_count, stt_cloud_count, llm_local_count, llm_cloud_count,
                        error_count
                 FROM usage_stats ORDER BY date DESC LIMIT ?1",
            )?;
            let rows = stmt.query_map([days], |r| {
                Ok(DailyStats {
                    date: r.get(0)?,
                    transcription_count: r.get(1)?,
                    total_words: r.get(2)?,
                    total_duration_ms: r.get(3)?,
                    stt_local_count: r.get(4)?,
                    stt_cloud_count: r.get(5)?,
                    llm_local_count: r.get(6)?,
                    llm_cloud_count: r.get(7)?,
                    error_count: r.get(8)?,
                })
            })?;
            Ok(rows.collect::<std::result::Result<Vec<_>, _>>()?)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_and_aggregates() {
        let db = Database::open_in_memory().unwrap();
        let repo = StatsRepository::new(&db);
        repo.record_transcription(5, 1000, EngineKind::Cloud, EngineKind::Local)
            .unwrap();
        repo.record_transcription(3, 500, EngineKind::Cloud, EngineKind::Local)
            .unwrap();
        repo.record_error().unwrap();

        let recent = repo.recent(7).unwrap();
        assert_eq!(recent.len(), 1);
        let today = &recent[0];
        assert_eq!(today.transcription_count, 2);
        assert_eq!(today.total_words, 8);
        assert_eq!(today.total_duration_ms, 1500);
        assert_eq!(today.stt_cloud_count, 2);
        assert_eq!(today.llm_local_count, 2);
        assert_eq!(today.error_count, 1);
    }
}
