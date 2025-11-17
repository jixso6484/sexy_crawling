use anyhow::Result;
use sqlx::SqlitePool;
use serde::{Deserialize, Serialize};
use tracing::{debug, info};
use crate::types::Product;

/// 데이터베이스 상품 (간소화)
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct DbProduct {
    pub id: i64,
    pub product_url: String,
    pub product_id: Option<String>,
    pub name: Option<String>,
    pub price: Option<String>,
    pub original_price: Option<String>,
    pub discount_rate: Option<String>,
    pub rating: Option<f64>,
    pub review_count: Option<i64>,
    pub image_url: Option<String>,
    pub seller: Option<String>,
    pub source: String,
    pub crawled_at: String,
    pub updated_at: String,
    pub is_processed: i64,
    pub raw_data: Option<String>,
}

/// 상품 관리자
pub struct ProductManager {
    pool: SqlitePool,
}

impl ProductManager {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// 상품 저장
    pub async fn save_product(&self, product: &Product) -> Result<i64> {
        debug!("Saving product: {}", product.name);

        // Product를 JSON으로 직렬화 (raw_data)
        let raw_data = serde_json::to_string(product)?;

        // 상품 ID 추출 (URL에서)
        let product_id = self.extract_product_id(&product.product_url);

        let result = sqlx::query(
            r#"
            INSERT OR REPLACE INTO products
            (product_url, product_id, name, price, original_price, discount_rate,
             rating, review_count, image_url, seller, source, raw_data, is_processed)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 0)
            "#
        )
        .bind(&product.product_url)
        .bind(product_id)
        .bind(&product.name)
        .bind(&product.price)
        .bind(&product.original_price)
        .bind(&product.discount_rate)
        .bind(product.rating.map(|r| r as f64))
        .bind(product.review_count.map(|r| r as i64))
        .bind(&product.image_url)
        .bind(&product.seller)
        .bind(product.source.as_str())
        .bind(raw_data)
        .execute(&self.pool)
        .await?;

        debug!("Product saved: {}", product.name);
        Ok(result.last_insert_rowid())
    }

    /// 여러 상품 일괄 저장 (스트리밍 방식 - 메모리 효율적)
    pub async fn save_products_batch(&self, products: &[Product]) -> Result<usize> {
        info!("Saving {} products", products.len());

        let mut saved = 0;
        for product in products {
            if self.save_product(product).await.is_ok() {
                saved += 1;
            }
        }

        info!("Saved {} products successfully", saved);
        Ok(saved)
    }

    /// 상품 ID 추출 (URL에서)
    fn extract_product_id(&self, url: &str) -> Option<String> {
        // 알리익스프레스 상품 ID 패턴: /item/{id}.html
        if let Some(start) = url.find("/item/") {
            let id_part = &url[start + 6..];
            if let Some(end) = id_part.find(".html") {
                return Some(id_part[..end].to_string());
            }
        }

        // 쿠팡: /products/{id}
        if let Some(start) = url.find("/products/") {
            let id_part = &url[start + 10..];
            if let Some(end) = id_part.find('?') {
                return Some(id_part[..end].to_string());
            } else {
                return Some(id_part.to_string());
            }
        }

        None
    }

    /// 상품 조회 (URL로)
    pub async fn get_product_by_url(&self, url: &str) -> Result<Option<DbProduct>> {
        let product = sqlx::query_as::<_, DbProduct>(
            "SELECT * FROM products WHERE product_url = ?"
        )
        .bind(url)
        .fetch_optional(&self.pool)
        .await?;

        Ok(product)
    }

    /// 미처리 상품 가져오기
    pub async fn get_unprocessed_products(&self, limit: i64) -> Result<Vec<DbProduct>> {
        let products = sqlx::query_as::<_, DbProduct>(
            r#"
            SELECT * FROM products
            WHERE is_processed = 0
            ORDER BY crawled_at ASC
            LIMIT ?
            "#
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        info!("Retrieved {} unprocessed products", products.len());
        Ok(products)
    }

    /// 상품을 처리 완료로 표시
    pub async fn mark_as_processed(&self, product_id: i64) -> Result<()> {
        sqlx::query("UPDATE products SET is_processed = 1, updated_at = datetime('now') WHERE id = ?")
            .bind(product_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// 상품 통계
    pub async fn get_product_stats(&self) -> Result<ProductStats> {
        let stats = sqlx::query_as::<_, ProductStats>(
            r#"
            SELECT
                COUNT(*) as total_products,
                COUNT(CASE WHEN is_processed = 1 THEN 1 END) as processed_products,
                COUNT(DISTINCT source) as unique_sources,
                AVG(rating) as avg_rating,
                COUNT(CASE WHEN rating IS NOT NULL THEN 1 END) as products_with_rating
            FROM products
            "#
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(stats)
    }

    /// 소스별 상품 수
    pub async fn get_products_by_source(&self) -> Result<Vec<SourceCount>> {
        let counts = sqlx::query_as::<_, SourceCount>(
            r#"
            SELECT source, COUNT(*) as count
            FROM products
            GROUP BY source
            ORDER BY count DESC
            "#
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(counts)
    }

    /// 상품 존재 여부 확인
    pub async fn product_exists(&self, url: &str) -> Result<bool> {
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM products WHERE product_url = ?")
            .bind(url)
            .fetch_one(&self.pool)
            .await?;

        Ok(count > 0)
    }
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ProductStats {
    pub total_products: i64,
    pub processed_products: i64,
    pub unique_sources: i64,
    pub avg_rating: Option<f64>,
    pub products_with_rating: i64,
}

impl ProductStats {
    pub fn print(&self) {
        println!("\n📦 상품 통계");
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!("  전체 상품:        {:>8}", self.total_products);
        println!("  처리된 상품:      {:>8}", self.processed_products);
        println!("  데이터 소스:      {:>8}", self.unique_sources);
        println!("  평균 평점:        {:>8.2}", self.avg_rating.unwrap_or(0.0));
        println!("  평점 있는 상품:   {:>8}", self.products_with_rating);
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
    }
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct SourceCount {
    pub source: String,
    pub count: i64,
}
