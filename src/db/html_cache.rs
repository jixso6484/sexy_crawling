use anyhow::{Result, Context};
use sqlx::SqlitePool;
use serde::{Deserialize, Serialize};
use tracing::{debug, info};
use flate2::Compression;
use flate2::write::GzEncoder;
use flate2::read::GzDecoder;
use std::io::{Write, Read};
use sha2::{Sha256, Digest};

/// HTML 캐시 항목
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct HtmlCacheItem {
    pub id: i64,
    pub url: String,
    pub html_content: Vec<u8>,  // 압축된 HTML
    pub content_hash: String,
    pub content_length: i64,
    pub compressed_length: i64,
    pub crawled_at: String,
    pub expires_at: Option<String>,
    pub is_valid: i64,
    pub error_message: Option<String>,
}

/// HTML 캐시 관리자
pub struct HtmlCache {
    pool: SqlitePool,
}

impl HtmlCache {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// HTML 압축
    fn compress_html(html: &str) -> Result<Vec<u8>> {
        let mut encoder = GzEncoder::new(Vec::new(), Compression::best());
        encoder.write_all(html.as_bytes())?;
        Ok(encoder.finish()?)
    }

    /// HTML 압축 해제
    fn decompress_html(compressed: &[u8]) -> Result<String> {
        let mut decoder = GzDecoder::new(compressed);
        let mut decompressed = String::new();
        decoder.read_to_string(&mut decompressed)?;
        Ok(decompressed)
    }

    /// SHA256 해시 계산
    fn calculate_hash(content: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// HTML 저장 (압축)
    pub async fn save_html(
        &self,
        url: &str,
        html: &str,
        expires_hours: Option<i64>,
    ) -> Result<i64> {
        debug!("Saving HTML cache for: {}", url);

        let compressed = Self::compress_html(html)
            .context("Failed to compress HTML")?;

        let content_hash = Self::calculate_hash(html);
        let content_length = html.len() as i64;
        let compressed_length = compressed.len() as i64;

        let compression_ratio = (1.0 - (compressed_length as f64 / content_length as f64)) * 100.0;
        debug!("Compression: {} -> {} bytes ({:.1}% saved)",
               content_length, compressed_length, compression_ratio);

        let expires_at = if let Some(hours) = expires_hours {
            Some(
                chrono::Utc::now()
                    .checked_add_signed(chrono::Duration::hours(hours))
                    .unwrap()
                    .format("%Y-%m-%d %H:%M:%S")
                    .to_string()
            )
        } else {
            None
        };

        let result = sqlx::query(
            r#"
            INSERT OR REPLACE INTO html_cache
            (url, html_content, content_hash, content_length, compressed_length, expires_at, is_valid)
            VALUES (?, ?, ?, ?, ?, ?, 1)
            "#
        )
        .bind(url)
        .bind(&compressed)
        .bind(&content_hash)
        .bind(content_length)
        .bind(compressed_length)
        .bind(expires_at)
        .execute(&self.pool)
        .await?;

        debug!("HTML cache saved successfully: {}", url);
        Ok(result.last_insert_rowid())
    }

    /// HTML 가져오기 (압축 해제)
    pub async fn get_html(&self, url: &str) -> Result<Option<String>> {
        debug!("Retrieving HTML cache for: {}", url);

        let item: Option<HtmlCacheItem> = sqlx::query_as(
            r#"
            SELECT * FROM html_cache
            WHERE url = ? AND is_valid = 1
            AND (expires_at IS NULL OR expires_at > datetime('now'))
            "#
        )
        .bind(url)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(item) = item {
            debug!("Cache hit: {}", url);
            let html = Self::decompress_html(&item.html_content)
                .context("Failed to decompress HTML")?;
            Ok(Some(html))
        } else {
            debug!("Cache miss: {}", url);
            Ok(None)
        }
    }

    /// 캐시 존재 여부 확인
    pub async fn has_cache(&self, url: &str) -> Result<bool> {
        let count: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*) FROM html_cache
            WHERE url = ? AND is_valid = 1
            AND (expires_at IS NULL OR expires_at > datetime('now'))
            "#
        )
        .bind(url)
        .fetch_one(&self.pool)
        .await?;

        Ok(count > 0)
    }

    /// 캐시 무효화
    pub async fn invalidate_cache(&self, url: &str) -> Result<()> {
        debug!("Invalidating cache for: {}", url);

        sqlx::query("UPDATE html_cache SET is_valid = 0 WHERE url = ?")
            .bind(url)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// 만료된 캐시 삭제
    pub async fn cleanup_expired(&self) -> Result<u64> {
        info!("Cleaning up expired HTML cache");

        let result = sqlx::query(
            r#"
            DELETE FROM html_cache
            WHERE expires_at IS NOT NULL AND expires_at < datetime('now')
            "#
        )
        .execute(&self.pool)
        .await?;

        info!("Cleaned up {} expired cache entries", result.rows_affected());
        Ok(result.rows_affected())
    }

    /// 캐시 통계
    pub async fn get_cache_stats(&self) -> Result<CacheStats> {
        let stats = sqlx::query_as::<_, CacheStats>(
            r#"
            SELECT
                COUNT(*) as total_entries,
                SUM(content_length) as total_original_size,
                SUM(compressed_length) as total_compressed_size,
                COUNT(CASE WHEN is_valid = 1 THEN 1 END) as valid_entries,
                COUNT(CASE WHEN expires_at < datetime('now') THEN 1 END) as expired_entries
            FROM html_cache
            "#
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(stats)
    }

    /// 중복 HTML 찾기 (같은 해시)
    pub async fn find_duplicates(&self) -> Result<Vec<DuplicateGroup>> {
        let duplicates = sqlx::query_as::<_, DuplicateGroup>(
            r#"
            SELECT content_hash, COUNT(*) as count
            FROM html_cache
            WHERE is_valid = 1
            GROUP BY content_hash
            HAVING count > 1
            ORDER BY count DESC
            "#
        )
        .fetch_all(&self.pool)
        .await?;

        info!("Found {} duplicate HTML groups", duplicates.len());
        Ok(duplicates)
    }
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct CacheStats {
    pub total_entries: i64,
    pub total_original_size: i64,
    pub total_compressed_size: i64,
    pub valid_entries: i64,
    pub expired_entries: i64,
}

impl CacheStats {
    pub fn print(&self) {
        let compression_ratio = if self.total_original_size > 0 {
            (1.0 - (self.total_compressed_size as f64 / self.total_original_size as f64)) * 100.0
        } else {
            0.0
        };

        println!("\n💾 HTML 캐시 통계");
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!("  전체 항목:        {:>8}", self.total_entries);
        println!("  유효한 항목:      {:>8}", self.valid_entries);
        println!("  만료된 항목:      {:>8}", self.expired_entries);
        println!("  원본 크기:        {:>8} KB", self.total_original_size / 1024);
        println!("  압축 크기:        {:>8} KB", self.total_compressed_size / 1024);
        println!("  압축률:           {:>7.1}%", compression_ratio);
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
    }
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct DuplicateGroup {
    pub content_hash: String,
    pub count: i64,
}
