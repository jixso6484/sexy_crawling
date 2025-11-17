use anyhow::Result;
use sqlx::SqlitePool;
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

/// URL 타입
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum UrlType {
    Search,
    Product,
    Category,
    Shop,
}

impl UrlType {
    pub fn as_str(&self) -> &str {
        match self {
            UrlType::Search => "search",
            UrlType::Product => "product",
            UrlType::Category => "category",
            UrlType::Shop => "shop",
        }
    }
}

/// URL 소스
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum UrlSource {
    Google,
    AliexpressDirect,
    Manual,
}

impl UrlSource {
    pub fn as_str(&self) -> &str {
        match self {
            UrlSource::Google => "google",
            UrlSource::AliexpressDirect => "aliexpress_direct",
            UrlSource::Manual => "manual",
        }
    }
}

/// URL 상태
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum UrlStatus {
    Pending,
    Processing,
    Completed,
    Failed,
    Skipped,
}

impl UrlStatus {
    pub fn as_str(&self) -> &str {
        match self {
            UrlStatus::Pending => "pending",
            UrlStatus::Processing => "processing",
            UrlStatus::Completed => "completed",
            UrlStatus::Failed => "failed",
            UrlStatus::Skipped => "skipped",
        }
    }
}

/// URL 큐 항목
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct UrlQueueItem {
    pub id: i64,
    pub url: String,
    pub url_type: String,
    pub source: String,
    pub status: String,
    pub priority: i64,
    pub search_query: Option<String>,
    pub discovered_at: String,
    pub first_attempt_at: Option<String>,
    pub last_attempt_at: Option<String>,
    pub completed_at: Option<String>,
    pub retry_count: i64,
    pub error_message: Option<String>,
    pub metadata: Option<String>,
}

/// URL 큐 관리자
pub struct UrlQueue {
    pool: SqlitePool,
}

impl UrlQueue {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// URL 추가 (중복 시 무시)
    pub async fn add_url(
        &self,
        url: &str,
        url_type: UrlType,
        source: UrlSource,
        priority: i64,
        search_query: Option<&str>,
        metadata: Option<&str>,
    ) -> Result<i64> {
        debug!("Adding URL to queue: {}", url);

        let result = sqlx::query(
            r#"
            INSERT OR IGNORE INTO url_queue (url, url_type, source, priority, search_query, metadata)
            VALUES (?, ?, ?, ?, ?, ?)
            "#
        )
        .bind(url)
        .bind(url_type.as_str())
        .bind(source.as_str())
        .bind(priority)
        .bind(search_query)
        .bind(metadata)
        .execute(&self.pool)
        .await?;

        if result.rows_affected() > 0 {
            debug!("URL added successfully: {}", url);
            Ok(result.last_insert_rowid())
        } else {
            debug!("URL already exists: {}", url);
            // 이미 존재하는 URL의 ID 가져오기
            let id: i64 = sqlx::query_scalar("SELECT id FROM url_queue WHERE url = ?")
                .bind(url)
                .fetch_one(&self.pool)
                .await?;
            Ok(id)
        }
    }

    /// 여러 URL 일괄 추가
    pub async fn add_urls_batch(
        &self,
        urls: &[(String, UrlType, UrlSource, i64, Option<String>)],
    ) -> Result<usize> {
        info!("Adding {} URLs to queue", urls.len());

        let mut added = 0;
        for (url, url_type, source, priority, search_query) in urls {
            match self.add_url(
                url,
                url_type.clone(),
                source.clone(),
                *priority,
                search_query.as_deref(),
                None,
            ).await {
                Ok(_) => added += 1,
                Err(e) => warn!("Failed to add URL {}: {}", url, e),
            }
        }

        info!("Added {} URLs successfully", added);
        Ok(added)
    }

    /// 다음 처리할 URL 가져오기 (우선순위 순)
    pub async fn get_next_url(&self) -> Result<Option<UrlQueueItem>> {
        let url = sqlx::query_as::<_, UrlQueueItem>(
            r#"
            SELECT * FROM url_queue
            WHERE status = 'pending'
            ORDER BY priority DESC, discovered_at ASC
            LIMIT 1
            "#
        )
        .fetch_optional(&self.pool)
        .await?;

        if let Some(ref item) = url {
            debug!("Next URL to process: {}", item.url);
        }

        Ok(url)
    }

    /// 여러 URL 가져오기 (배치 처리)
    pub async fn get_next_urls(&self, limit: i64) -> Result<Vec<UrlQueueItem>> {
        let urls = sqlx::query_as::<_, UrlQueueItem>(
            r#"
            SELECT * FROM url_queue
            WHERE status = 'pending'
            ORDER BY priority DESC, discovered_at ASC
            LIMIT ?
            "#
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        info!("Retrieved {} pending URLs", urls.len());
        Ok(urls)
    }

    /// URL 상태 업데이트
    pub async fn update_status(
        &self,
        id: i64,
        status: UrlStatus,
        error_message: Option<&str>,
    ) -> Result<()> {
        debug!("Updating URL status: id={}, status={}", id, status.as_str());

        let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();

        match status {
            UrlStatus::Processing => {
                sqlx::query(
                    r#"
                    UPDATE url_queue
                    SET status = ?,
                        first_attempt_at = COALESCE(first_attempt_at, ?),
                        last_attempt_at = ?,
                        retry_count = retry_count + 1
                    WHERE id = ?
                    "#
                )
                .bind(status.as_str())
                .bind(&now)
                .bind(&now)
                .bind(id)
                .execute(&self.pool)
                .await?;
            }
            UrlStatus::Completed => {
                sqlx::query(
                    r#"
                    UPDATE url_queue
                    SET status = ?,
                        completed_at = ?,
                        error_message = NULL
                    WHERE id = ?
                    "#
                )
                .bind(status.as_str())
                .bind(&now)
                .bind(id)
                .execute(&self.pool)
                .await?;
            }
            UrlStatus::Failed | UrlStatus::Skipped => {
                sqlx::query(
                    r#"
                    UPDATE url_queue
                    SET status = ?,
                        error_message = ?,
                        last_attempt_at = ?
                    WHERE id = ?
                    "#
                )
                .bind(status.as_str())
                .bind(error_message)
                .bind(&now)
                .bind(id)
                .execute(&self.pool)
                .await?;
            }
            UrlStatus::Pending => {
                sqlx::query("UPDATE url_queue SET status = ? WHERE id = ?")
                    .bind(status.as_str())
                    .bind(id)
                    .execute(&self.pool)
                    .await?;
            }
        }

        Ok(())
    }

    /// 실패한 URL 재시도 (상태를 pending으로 변경)
    pub async fn retry_failed_urls(&self, max_retry: i64) -> Result<u64> {
        info!("Retrying failed URLs (max_retry={})", max_retry);

        let result = sqlx::query(
            r#"
            UPDATE url_queue
            SET status = 'pending', error_message = NULL
            WHERE status = 'failed' AND retry_count < ?
            "#
        )
        .bind(max_retry)
        .execute(&self.pool)
        .await?;

        info!("Retrying {} failed URLs", result.rows_affected());
        Ok(result.rows_affected())
    }

    /// URL 존재 여부 확인
    pub async fn url_exists(&self, url: &str) -> Result<bool> {
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM url_queue WHERE url = ?")
            .bind(url)
            .fetch_one(&self.pool)
            .await?;

        Ok(count > 0)
    }

    /// 큐 통계
    pub async fn get_queue_stats(&self) -> Result<QueueStats> {
        let stats = sqlx::query_as::<_, QueueStats>(
            r#"
            SELECT
                COUNT(*) as total,
                COUNT(CASE WHEN status = 'pending' THEN 1 END) as pending,
                COUNT(CASE WHEN status = 'processing' THEN 1 END) as processing,
                COUNT(CASE WHEN status = 'completed' THEN 1 END) as completed,
                COUNT(CASE WHEN status = 'failed' THEN 1 END) as failed,
                COUNT(CASE WHEN status = 'skipped' THEN 1 END) as skipped
            FROM url_queue
            "#
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(stats)
    }
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct QueueStats {
    pub total: i64,
    pub pending: i64,
    pub processing: i64,
    pub completed: i64,
    pub failed: i64,
    pub skipped: i64,
}

impl QueueStats {
    pub fn print(&self) {
        println!("\n📋 URL 큐 통계");
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!("  전체:      {:>8}", self.total);
        println!("  대기:      {:>8}", self.pending);
        println!("  처리 중:   {:>8}", self.processing);
        println!("  완료:      {:>8}", self.completed);
        println!("  실패:      {:>8}", self.failed);
        println!("  건너뜀:    {:>8}", self.skipped);
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
    }
}
