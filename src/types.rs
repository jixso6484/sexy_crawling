use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Product {
    pub name: String,
    pub price: Option<String>,
    pub original_price: Option<String>,
    pub discount_rate: Option<String>,
    pub rating: Option<f32>,
    pub review_count: Option<u32>,
    pub image_url: Option<String>,
    pub product_url: String,
    pub seller: Option<String>,
    pub delivery_info: Option<String>,
    pub source: ProductSource,
    pub raw_data: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProductSource {
    Coupang,
    Danawa,
    AliExpress,
}

impl std::fmt::Display for ProductSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProductSource::Coupang => write!(f, "쿠팡"),
            ProductSource::Danawa => write!(f, "다나와"),
            ProductSource::AliExpress => write!(f, "알리익스프레스"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessedProduct {
    pub original: Product,
    pub normalized_name: String,
    pub normalized_price: Option<f64>,
    pub category: Option<String>,
    pub features: Vec<String>,
    pub summary: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrawlConfig {
    pub search_query: String,
    pub category_url: Option<String>,  // 카테고리 URL (다나와 전용)
    pub max_pages: u32,
    pub crawl_all_pages: bool,  // true면 모든 페이지를 크롤링 (빈 페이지까지)
    pub timeout_secs: u64,
    pub user_agent: String,
    pub output_dir: Option<String>,  // HTML 및 중간 결과 저장 디렉토리
}

impl Default for CrawlConfig {
    fn default() -> Self {
        Self {
            search_query: String::new(),
            category_url: None,
            max_pages: 5, // 더 많은 데이터 수집을 위해 5페이지로 증가
            crawl_all_pages: false,  // 기본값은 max_pages만큼만 크롤링
            timeout_secs: 30,
            user_agent: "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36".to_string(),
            output_dir: None,
        }
    }
}
