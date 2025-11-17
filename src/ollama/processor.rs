use tracing::{debug, info, warn};

use crate::error::Result;
use crate::ollama::OllamaClient;
use crate::types::{ProcessedProduct, Product};

pub struct ProductProcessor {
    ollama_client: OllamaClient,
}

impl ProductProcessor {
    pub fn new(ollama_client: OllamaClient) -> Self {
        Self { ollama_client }
    }

    /// 단일 상품 정보를 Ollama로 처리하여 정규화
    pub async fn process_product(&self, product: &Product) -> Result<ProcessedProduct> {
        debug!("Processing product: {}", product.name);

        let prompt = self.build_product_analysis_prompt(product);
        let response = self.ollama_client.generate(&prompt).await?;

        let processed = self.parse_ollama_response(product, &response);

        info!("Processed product: {}", product.name);

        Ok(processed)
    }

    /// 여러 상품을 일괄 처리
    pub async fn process_products(&self, products: &[Product]) -> Vec<ProcessedProduct> {
        let mut processed_products = Vec::new();

        for (idx, product) in products.iter().enumerate() {
            info!("Processing product {}/{}", idx + 1, products.len());

            match self.process_product(product).await {
                Ok(processed) => {
                    processed_products.push(processed);
                }
                Err(e) => {
                    warn!("Failed to process product '{}': {}", product.name, e);
                    // 실패한 경우 기본 처리된 상품 생성
                    processed_products.push(ProcessedProduct {
                        original: product.clone(),
                        normalized_name: product.name.clone(),
                        normalized_price: self.extract_price(&product.price),
                        category: None,
                        features: Vec::new(),
                        summary: None,
                    });
                }
            }

            // API 요청 제한을 위한 딜레이
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }

        processed_products
    }

    /// 상품 비교 분석 (여러 상품을 비교)
    pub async fn compare_products(&self, products: &[Product]) -> Result<String> {
        info!("Comparing {} products", products.len());

        let prompt = self.build_comparison_prompt(products);
        let response = self.ollama_client.generate(&prompt).await?;

        Ok(response)
    }

    /// 상품 요약 생성
    pub async fn summarize_products(&self, products: &[Product]) -> Result<String> {
        info!("Summarizing {} products", products.len());

        let prompt = self.build_summary_prompt(products);
        let response = self.ollama_client.generate(&prompt).await?;

        Ok(response)
    }

    // Private helper methods

    fn build_product_analysis_prompt(&self, product: &Product) -> String {
        format!(
            r#"다음 상품 정보를 분석하여 정규화된 형식으로 정리해주세요.

상품명: {}
가격: {}
출처: {}
평점: {}
리뷰 수: {}

다음 형식으로 JSON 응답을 제공해주세요:
{{
  "normalized_name": "정규화된 상품명",
  "normalized_price": 숫자_가격,
  "category": "상품_카테고리",
  "features": ["특징1", "특징2", "특징3"],
  "summary": "상품에 대한 간단한 요약"
}}

특히:
1. normalized_name: 브랜드명, 모델명, 주요 스펙이 포함된 정리된 상품명
2. normalized_price: 숫자만 추출 (원화 기준)
3. category: 전자제품, 가전, 패션, 식품 등의 카테고리
4. features: 상품명에서 추출한 주요 특징들
5. summary: 한 줄로 정리한 상품 설명

JSON만 응답하고 다른 설명은 생략해주세요."#,
            product.name,
            product.price.as_deref().unwrap_or("정보 없음"),
            product.source,
            product.rating.map(|r| r.to_string()).unwrap_or_else(|| "정보 없음".to_string()),
            product.review_count.map(|c| c.to_string()).unwrap_or_else(|| "정보 없음".to_string())
        )
    }

    fn build_comparison_prompt(&self, products: &[Product]) -> String {
        let mut prompt = String::from("다음 상품들을 비교 분석해주세요:\n\n");

        for (idx, product) in products.iter().enumerate() {
            prompt.push_str(&format!(
                "{}. {} ({}원) - {}\n",
                idx + 1,
                product.name,
                product.price.as_deref().unwrap_or("가격 정보 없음"),
                product.source
            ));
        }

        prompt.push_str("\n다음 항목을 포함하여 분석해주세요:\n");
        prompt.push_str("1. 가격 비교 (가장 저렴한 옵션)\n");
        prompt.push_str("2. 각 판매처의 장단점\n");
        prompt.push_str("3. 추천 의견\n");
        prompt.push_str("4. 주의사항\n");

        prompt
    }

    fn build_summary_prompt(&self, products: &[Product]) -> String {
        let mut prompt = String::from("다음 검색 결과를 요약해주세요:\n\n");

        let sample_count = products.len().min(10);
        for (idx, product) in products.iter().take(sample_count).enumerate() {
            prompt.push_str(&format!(
                "{}. {} - {}원 ({})\n",
                idx + 1,
                product.name,
                product.price.as_deref().unwrap_or("가격 정보 없음"),
                product.source
            ));
        }

        if products.len() > sample_count {
            prompt.push_str(&format!("\n... 외 {}개 상품\n", products.len() - sample_count));
        }

        prompt.push_str("\n다음 내용을 포함하여 요약해주세요:\n");
        prompt.push_str("1. 전체적인 가격대 분석\n");
        prompt.push_str("2. 주요 판매처 분포\n");
        prompt.push_str("3. 특이사항이나 주목할 만한 상품\n");

        prompt
    }

    fn parse_ollama_response(&self, product: &Product, response: &str) -> ProcessedProduct {
        // JSON 파싱 시도
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(response) {
            ProcessedProduct {
                original: product.clone(),
                normalized_name: json["normalized_name"]
                    .as_str()
                    .unwrap_or(&product.name)
                    .to_string(),
                normalized_price: json["normalized_price"]
                    .as_f64()
                    .or_else(|| self.extract_price(&product.price)),
                category: json["category"].as_str().map(|s| s.to_string()),
                features: json["features"]
                    .as_array()
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(|s| s.to_string()))
                            .collect()
                    })
                    .unwrap_or_default(),
                summary: json["summary"].as_str().map(|s| s.to_string()),
            }
        } else {
            // JSON 파싱 실패 시 기본값 사용
            warn!("Failed to parse Ollama response as JSON, using defaults");
            ProcessedProduct {
                original: product.clone(),
                normalized_name: product.name.clone(),
                normalized_price: self.extract_price(&product.price),
                category: None,
                features: Vec::new(),
                summary: Some(response.to_string()),
            }
        }
    }

    fn extract_price(&self, price_str: &Option<String>) -> Option<f64> {
        price_str.as_ref().and_then(|p| {
            let cleaned: String = p.chars().filter(|c| c.is_numeric() || *c == '.').collect();
            cleaned.parse::<f64>().ok()
        })
    }
}
