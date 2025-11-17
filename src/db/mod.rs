pub mod schema;
pub mod url_queue;
pub mod html_cache;
pub mod products;
pub mod sessions;

use anyhow::Result;
use sqlx::{sqlite::SqlitePool, SqliteConnection};
use std::path::Path;
use tracing::{info, debug};

/// SQLite 데이터베이스 관리자
pub struct Database {
    pool: SqlitePool,
}

impl Database {
    /// 데이터베이스 연결 및 초기화
    ///
    /// # Arguments
    /// * `db_path` - SQLite 데이터베이스 파일 경로 (예: "crawl_data.db")
    pub async fn new(db_path: &str) -> Result<Self> {
        info!("Initializing database at: {}", db_path);

        // 데이터베이스 파일이 없으면 생성됨
        let db_url = format!("sqlite:{}", db_path);

        // 연결 풀 생성
        let pool = SqlitePool::connect(&db_url).await?;

        info!("Database connection established");

        let db = Self { pool };

        // 스키마 초기화
        db.init_schema().await?;

        Ok(db)
    }

    /// 스키마 초기화 (테이블 생성)
    async fn init_schema(&self) -> Result<()> {
        info!("Initializing database schema");

        // schema.sql 파일의 내용을 실행
        let schema_sql = include_str!("../../schema.sql");

        // SQL을 세미콜론으로 분리하여 실행
        for statement in schema_sql.split(';') {
            let trimmed = statement.trim();
            if !trimmed.is_empty() && !trimmed.starts_with("--") {
                sqlx::query(trimmed)
                    .execute(&self.pool)
                    .await?;
            }
        }

        info!("Database schema initialized successfully");
        Ok(())
    }

    /// 연결 풀 가져오기
    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    /// 데이터베이스 통계 가져오기
    pub async fn get_stats(&self) -> Result<CrawlStats> {
        let stats = sqlx::query_as::<_, CrawlStats>(
            r#"
            SELECT
                COUNT(CASE WHEN status = 'pending' THEN 1 END) as pending_urls,
                COUNT(CASE WHEN status = 'completed' THEN 1 END) as completed_urls,
                COUNT(CASE WHEN status = 'failed' THEN 1 END) as failed_urls,
                COUNT(CASE WHEN status = 'processing' THEN 1 END) as processing_urls,
                (SELECT COUNT(*) FROM products) as total_products,
                (SELECT COUNT(*) FROM products WHERE is_processed = 1) as processed_products,
                (SELECT COUNT(*) FROM crawl_sessions WHERE status = 'running') as active_sessions
            FROM url_queue
            "#
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(stats)
    }

    /// 데이터베이스 정리 (오래된 캐시 삭제)
    pub async fn cleanup(&self, days: i64) -> Result<u64> {
        info!("Cleaning up old cache data (older than {} days)", days);

        let result = sqlx::query(
            r#"
            DELETE FROM html_cache
            WHERE expires_at < datetime('now', '-' || ? || ' days')
            "#
        )
        .bind(days)
        .execute(&self.pool)
        .await?;

        info!("Cleaned up {} old cache entries", result.rows_affected());
        Ok(result.rows_affected())
    }

    /// 데이터베이스 최적화 (VACUUM)
    pub async fn optimize(&self) -> Result<()> {
        info!("Optimizing database");
        sqlx::query("VACUUM").execute(&self.pool).await?;
        info!("Database optimized");
        Ok(())
    }
}

/// 크롤링 통계
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct CrawlStats {
    pub pending_urls: i64,
    pub completed_urls: i64,
    pub failed_urls: i64,
    pub processing_urls: i64,
    pub total_products: i64,
    pub processed_products: i64,
    pub active_sessions: i64,
}

impl CrawlStats {
    pub fn print(&self) {
        println!("\n📊 크롤링 통계");
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!("  대기 중인 URL:    {:>8}", self.pending_urls);
        println!("  처리 중인 URL:    {:>8}", self.processing_urls);
        println!("  완료된 URL:       {:>8}", self.completed_urls);
        println!("  실패한 URL:       {:>8}", self.failed_urls);
        println!("  전체 상품:        {:>8}", self.total_products);
        println!("  처리된 상품:      {:>8}", self.processed_products);
        println!("  활성 세션:        {:>8}", self.active_sessions);
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_database_init() {
        let db = Database::new(":memory:").await.unwrap();
        let stats = db.get_stats().await.unwrap();
        assert_eq!(stats.pending_urls, 0);
    }
}
