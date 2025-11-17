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

impl ProductSource {
    pub fn as_str(&self) -> &str {
        match self {
            ProductSource::Coupang => "coupang",
            ProductSource::Danawa => "danawa",
            ProductSource::AliExpress => "aliexpress",
        }
    }
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
    pub max_pages: u32,
    pub timeout_secs: u64,
    pub user_agent: String,
}

impl Default for CrawlConfig {
    fn default() -> Self {
        Self {
            search_query: String::new(),
            max_pages: 5, // 더 많은 데이터 수집을 위해 5페이지로 증가
            timeout_secs: 30,
            user_agent: "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36".to_string(),
        }
    }
}
