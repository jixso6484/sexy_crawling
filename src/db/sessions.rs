use anyhow::Result;
use sqlx::SqlitePool;
use serde::{Deserialize, Serialize};
use tracing::{debug, info};
use uuid::Uuid;

/// 크롤링 모드
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum CrawlMode {
    Google,
    Direct,
    Hybrid,
}

impl CrawlMode {
    pub fn as_str(&self) -> &str {
        match self {
            CrawlMode::Google => "google",
            CrawlMode::Direct => "direct",
            CrawlMode::Hybrid => "hybrid",
        }
    }
}

/// 세션 상태
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum SessionStatus {
    Running,
    Paused,
    Completed,
    Failed,
}

impl SessionStatus {
    pub fn as_str(&self) -> &str {
        match self {
            SessionStatus::Running => "running",
            SessionStatus::Paused => "paused",
            SessionStatus::Completed => "completed",
            SessionStatus::Failed => "failed",
        }
    }
}

/// 크롤링 세션
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct CrawlSession {
    pub id: i64,
    pub session_id: String,
    pub crawl_mode: String,
    pub search_query: Option<String>,
    pub started_at: String,
    pub finished_at: Option<String>,
    pub total_urls_discovered: i64,
    pub total_urls_crawled: i64,
    pub total_products_found: i64,
    pub total_errors: i64,
    pub status: String,
    pub config: Option<String>,
}

/// 세션 관리자
pub struct SessionManager {
    pool: SqlitePool,
}

impl SessionManager {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// 새 세션 시작
    pub async fn start_session(
        &self,
        crawl_mode: CrawlMode,
        search_query: Option<&str>,
        config: Option<&str>,
    ) -> Result<String> {
        let session_id = Uuid::new_v4().to_string();
        info!("Starting new crawl session: {}", session_id);

        sqlx::query(
            r#"
            INSERT INTO crawl_sessions (session_id, crawl_mode, search_query, config, status)
            VALUES (?, ?, ?, ?, 'running')
            "#
        )
        .bind(&session_id)
        .bind(crawl_mode.as_str())
        .bind(search_query)
        .bind(config)
        .execute(&self.pool)
        .await?;

        info!("Session started: {}", session_id);
        Ok(session_id)
    }

    /// 세션 종료
    pub async fn finish_session(&self, session_id: &str, status: SessionStatus) -> Result<()> {
        info!("Finishing session: {} with status: {}", session_id, status.as_str());

        let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();

        sqlx::query(
            "UPDATE crawl_sessions SET finished_at = ?, status = ? WHERE session_id = ?"
        )
        .bind(&now)
        .bind(status.as_str())
        .bind(session_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// 세션 통계 업데이트
    pub async fn update_session_stats(
        &self,
        session_id: &str,
        urls_discovered: i64,
        urls_crawled: i64,
        products_found: i64,
        errors: i64,
    ) -> Result<()> {
        debug!("Updating session stats: {}", session_id);

        sqlx::query(
            r#"
            UPDATE crawl_sessions
            SET total_urls_discovered = total_urls_discovered + ?,
                total_urls_crawled = total_urls_crawled + ?,
                total_products_found = total_products_found + ?,
                total_errors = total_errors + ?
            WHERE session_id = ?
            "#
        )
        .bind(urls_discovered)
        .bind(urls_crawled)
        .bind(products_found)
        .bind(errors)
        .bind(session_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// 세션 조회
    pub async fn get_session(&self, session_id: &str) -> Result<Option<CrawlSession>> {
        let session = sqlx::query_as::<_, CrawlSession>(
            "SELECT * FROM crawl_sessions WHERE session_id = ?"
        )
        .bind(session_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(session)
    }

    /// 활성 세션 목록
    pub async fn get_active_sessions(&self) -> Result<Vec<CrawlSession>> {
        let sessions = sqlx::query_as::<_, CrawlSession>(
            "SELECT * FROM crawl_sessions WHERE status = 'running' ORDER BY started_at DESC"
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(sessions)
    }

    /// 최근 세션 목록
    pub async fn get_recent_sessions(&self, limit: i64) -> Result<Vec<CrawlSession>> {
        let sessions = sqlx::query_as::<_, CrawlSession>(
            "SELECT * FROM crawl_sessions ORDER BY started_at DESC LIMIT ?"
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(sessions)
    }

    /// 로그 추가
    pub async fn add_log(
        &self,
        session_id: &str,
        url: Option<&str>,
        log_level: &str,
        message: &str,
        error_details: Option<&str>,
    ) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO crawl_logs (session_id, url, log_level, message, error_details)
            VALUES (?, ?, ?, ?, ?)
            "#
        )
        .bind(session_id)
        .bind(url)
        .bind(log_level)
        .bind(message)
        .bind(error_details)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// 세션 로그 조회
    pub async fn get_session_logs(
        &self,
        session_id: &str,
        limit: Option<i64>,
    ) -> Result<Vec<CrawlLog>> {
        let logs = if let Some(limit) = limit {
            sqlx::query_as::<_, CrawlLog>(
                "SELECT * FROM crawl_logs WHERE session_id = ? ORDER BY created_at DESC LIMIT ?"
            )
            .bind(session_id)
            .bind(limit)
            .fetch_all(&self.pool)
            .await?
        } else {
            sqlx::query_as::<_, CrawlLog>(
                "SELECT * FROM crawl_logs WHERE session_id = ? ORDER BY created_at DESC"
            )
            .bind(session_id)
            .fetch_all(&self.pool)
            .await?
        };

        Ok(logs)
    }
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct CrawlLog {
    pub id: i64,
    pub session_id: Option<String>,
    pub url: Option<String>,
    pub log_level: Option<String>,
    pub message: Option<String>,
    pub error_details: Option<String>,
    pub created_at: String,
}
