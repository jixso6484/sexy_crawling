use sexy_crawling::{
    CoupangCrawler, CrawlConfig, Crawler, OllamaClient, ProductProcessor,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 로깅 초기화
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    // 크롤 설정
    let config = CrawlConfig {
        search_query: "무선 마우스".to_string(),
        max_pages: 2,
        ..Default::default()
    };

    // 쿠팡 크롤러 생성 및 실행
    println!("=== 쿠팡에서 상품 검색 ===");
    let crawler = CoupangCrawler::new(&config.user_agent)?;
    let products = crawler.crawl(&config).await?;

    println!("발견된 상품 수: {}", products.len());

    // 처음 5개 상품만 출력
    for (idx, product) in products.iter().take(5).enumerate() {
        println!(
            "{}. {} - {}",
            idx + 1,
            product.name,
            product.price.as_deref().unwrap_or("가격 정보 없음")
        );
    }

    // Ollama 클라이언트 생성
    println!("\n=== Ollama로 데이터 처리 ===");
    let ollama_client = OllamaClient::default();

    // 연결 확인
    if !ollama_client.check_connection().await? {
        println!("Ollama에 연결할 수 없습니다. 'ollama serve'를 실행하세요.");
        return Ok(());
    }

    // 상품 처리
    let processor = ProductProcessor::new(ollama_client);

    // 첫 번째 상품만 처리 (예제이므로)
    if let Some(product) = products.first() {
        println!("\n처리 중: {}", product.name);
        let processed = processor.process_product(product).await?;

        println!("\n처리 결과:");
        println!("  정규화된 이름: {}", processed.normalized_name);
        println!(
            "  정규화된 가격: {}",
            processed
                .normalized_price
                .map(|p| format!("{}원", p))
                .unwrap_or_else(|| "정보 없음".to_string())
        );
        if let Some(category) = processed.category {
            println!("  카테고리: {}", category);
        }
        if !processed.features.is_empty() {
            println!("  특징: {}", processed.features.join(", "));
        }
        if let Some(summary) = processed.summary {
            println!("  요약: {}", summary);
        }
    }

    Ok(())
}
