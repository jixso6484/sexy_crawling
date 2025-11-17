use clap::{Parser, Subcommand};
use sexy_crawling::{
    AliExpressCrawler, CoupangCrawler, CrawlConfig, Crawler, DanawaCrawler, OllamaClient,
    ProductProcessor,
};
use std::path::PathBuf;
use tracing::{error, info};
use tracing_subscriber;

#[derive(Parser)]
#[command(name = "sexy_crawling")]
#[command(author = "Your Name")]
#[command(version = "0.1.0")]
#[command(about = "쿠팡, 다나와, 알리익스프레스 크롤러 with Ollama", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Ollama 서버 URL
    #[arg(long, default_value = "http://localhost:11434")]
    ollama_url: String,

    /// Ollama 모델명
    #[arg(long, default_value = "llama2")]
    ollama_model: String,

    /// 최대 페이지 수
    #[arg(short, long, default_value = "3")]
    max_pages: u32,

    /// 상세 로그 출력
    #[arg(short, long)]
    verbose: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// 검색어로 상품 크롤링
    Search {
        /// 검색어
        query: String,

        /// 크롤링할 사이트 (coupang, danawa, aliexpress, all)
        #[arg(short, long, default_value = "all")]
        site: String,

        /// 결과를 저장할 파일 (JSON)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// 크롤링 결과를 Ollama로 분석
    Analyze {
        /// 분석할 JSON 파일
        input: PathBuf,

        /// 분석 타입 (process, compare, summarize)
        #[arg(short, long, default_value = "process")]
        analysis_type: String,

        /// 결과를 저장할 파일
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// 검색부터 분석까지 한번에
    Auto {
        /// 검색어
        query: String,

        /// 크롤링할 사이트 (coupang, danawa, aliexpress, all)
        #[arg(short, long, default_value = "all")]
        site: String,

        /// 결과를 저장할 파일 (JSON)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Ollama 연결 테스트
    Test,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // 로깅 설정
    if cli.verbose {
        tracing_subscriber::fmt()
            .with_max_level(tracing::Level::DEBUG)
            .init();
    } else {
        tracing_subscriber::fmt()
            .with_max_level(tracing::Level::INFO)
            .init();
    }

    // Ollama 클라이언트 생성
    let ollama_client = OllamaClient::new(cli.ollama_url.clone(), cli.ollama_model.clone());

    match cli.command {
        Commands::Search {
            query,
            site,
            output,
        } => {
            handle_search(&query, &site, cli.max_pages, output).await?;
        }
        Commands::Analyze {
            input,
            analysis_type,
            output,
        } => {
            handle_analyze(&ollama_client, input, &analysis_type, output).await?;
        }
        Commands::Auto {
            query,
            site,
            output,
        } => {
            handle_auto(&ollama_client, &query, &site, cli.max_pages, output).await?;
        }
        Commands::Test => {
            handle_test(&ollama_client).await?;
        }
    }

    Ok(())
}

async fn handle_search(
    query: &str,
    site: &str,
    max_pages: u32,
    output: Option<PathBuf>,
) -> anyhow::Result<()> {
    info!("검색 시작: '{}' (사이트: {})", query, site);

    let config = CrawlConfig {
        search_query: query.to_string(),
        max_pages,
        ..Default::default()
    };

    let mut all_products = Vec::new();

    // 사이트별 크롤링
    if site == "all" || site == "coupang" {
        info!("쿠팡 크롤링 시작...");
        match CoupangCrawler::new(&config.user_agent) {
            Ok(crawler) => match crawler.crawl(&config).await {
                Ok(mut products) => {
                    info!("쿠팡에서 {}개 상품 발견", products.len());
                    all_products.append(&mut products);
                }
                Err(e) => error!("쿠팡 크롤링 실패: {}", e),
            },
            Err(e) => error!("쿠팡 크롤러 생성 실패: {}", e),
        }
    }

    if site == "all" || site == "danawa" {
        info!("다나와 크롤링 시작...");
        match DanawaCrawler::new(&config.user_agent) {
            Ok(crawler) => match crawler.crawl(&config).await {
                Ok(mut products) => {
                    info!("다나와에서 {}개 상품 발견", products.len());
                    all_products.append(&mut products);
                }
                Err(e) => error!("다나와 크롤링 실패: {}", e),
            },
            Err(e) => error!("다나와 크롤러 생성 실패: {}", e),
        }
    }

    if site == "all" || site == "aliexpress" {
        info!("알리익스프레스 크롤링 시작...");
        match AliExpressCrawler::new(&config.user_agent) {
            Ok(crawler) => match crawler.crawl(&config).await {
                Ok(mut products) => {
                    info!("알리익스프레스에서 {}개 상품 발견", products.len());
                    all_products.append(&mut products);
                }
                Err(e) => error!("알리익스프레스 크롤링 실패: {}", e),
            },
            Err(e) => error!("알리익스프레스 크롤러 생성 실패: {}", e),
        }
    }

    info!("총 {}개 상품 발견", all_products.len());

    // 결과 출력 또는 저장
    if let Some(output_path) = output {
        let json = serde_json::to_string_pretty(&all_products)?;
        std::fs::write(&output_path, json)?;
        info!("결과를 {:?}에 저장했습니다", output_path);
    } else {
        // 콘솔에 간단히 출력
        for (idx, product) in all_products.iter().enumerate() {
            println!("{}. {} - {} ({})",
                idx + 1,
                product.name,
                product.price.as_deref().unwrap_or("가격 정보 없음"),
                product.source
            );
        }
    }

    Ok(())
}

async fn handle_analyze(
    ollama_client: &OllamaClient,
    input: PathBuf,
    analysis_type: &str,
    output: Option<PathBuf>,
) -> anyhow::Result<()> {
    info!("분석 시작: {:?} (타입: {})", input, analysis_type);

    // JSON 파일 읽기
    let json_content = std::fs::read_to_string(&input)?;
    let products: Vec<sexy_crawling::Product> = serde_json::from_str(&json_content)?;

    info!("{}개 상품 로드 완료", products.len());

    let processor = ProductProcessor::new(ollama_client.clone());

    match analysis_type {
        "process" => {
            info!("상품 처리 중...");
            let processed = processor.process_products(&products).await;

            if let Some(output_path) = output {
                let json = serde_json::to_string_pretty(&processed)?;
                std::fs::write(&output_path, json)?;
                info!("처리 결과를 {:?}에 저장했습니다", output_path);
            } else {
                for (idx, product) in processed.iter().enumerate() {
                    println!("{}. {} - {}",
                        idx + 1,
                        product.normalized_name,
                        product.summary.as_deref().unwrap_or("요약 없음")
                    );
                }
            }
        }
        "compare" => {
            info!("상품 비교 중...");
            let comparison = processor.compare_products(&products).await?;

            if let Some(output_path) = output {
                std::fs::write(&output_path, &comparison)?;
                info!("비교 결과를 {:?}에 저장했습니다", output_path);
            } else {
                println!("\n=== 상품 비교 분석 ===\n{}", comparison);
            }
        }
        "summarize" => {
            info!("상품 요약 중...");
            let summary = processor.summarize_products(&products).await?;

            if let Some(output_path) = output {
                std::fs::write(&output_path, &summary)?;
                info!("요약 결과를 {:?}에 저장했습니다", output_path);
            } else {
                println!("\n=== 상품 요약 ===\n{}", summary);
            }
        }
        _ => {
            error!("알 수 없는 분석 타입: {}", analysis_type);
            println!("사용 가능한 분석 타입: process, compare, summarize");
        }
    }

    Ok(())
}

async fn handle_auto(
    ollama_client: &OllamaClient,
    query: &str,
    site: &str,
    max_pages: u32,
    output: Option<PathBuf>,
) -> anyhow::Result<()> {
    info!("자동 모드: 크롤링 + 분석");

    // 1. 크롤링
    let config = CrawlConfig {
        search_query: query.to_string(),
        max_pages,
        ..Default::default()
    };

    let mut all_products = Vec::new();

    if site == "all" || site == "coupang" {
        info!("쿠팡 크롤링 시작...");
        if let Ok(crawler) = CoupangCrawler::new(&config.user_agent) {
            if let Ok(mut products) = crawler.crawl(&config).await {
                all_products.append(&mut products);
            }
        }
    }

    if site == "all" || site == "danawa" {
        info!("다나와 크롤링 시작...");
        if let Ok(crawler) = DanawaCrawler::new(&config.user_agent) {
            if let Ok(mut products) = crawler.crawl(&config).await {
                all_products.append(&mut products);
            }
        }
    }

    if site == "all" || site == "aliexpress" {
        info!("알리익스프레스 크롤링 시작...");
        if let Ok(crawler) = AliExpressCrawler::new(&config.user_agent) {
            if let Ok(mut products) = crawler.crawl(&config).await {
                all_products.append(&mut products);
            }
        }
    }

    info!("총 {}개 상품 발견", all_products.len());

    if all_products.is_empty() {
        println!("크롤링된 상품이 없습니다.");
        return Ok(());
    }

    // 2. Ollama로 처리
    info!("Ollama로 데이터 분석 중...");
    let processor = ProductProcessor::new(ollama_client.clone());
    let processed = processor.process_products(&all_products).await;

    // 3. 요약 생성
    let summary = processor.summarize_products(&all_products).await?;

    // 4. 결과 출력
    if let Some(output_path) = output {
        let result = serde_json::json!({
            "raw_products": all_products,
            "processed_products": processed,
            "summary": summary
        });
        let json = serde_json::to_string_pretty(&result)?;
        std::fs::write(&output_path, json)?;
        info!("결과를 {:?}에 저장했습니다", output_path);
    } else {
        println!("\n=== 처리된 상품 ===");
        for (idx, product) in processed.iter().take(10).enumerate() {
            println!("{}. {} - {}원",
                idx + 1,
                product.normalized_name,
                product.normalized_price.map(|p| p.to_string()).unwrap_or_else(|| "가격 정보 없음".to_string())
            );
        }

        println!("\n=== 요약 ===\n{}", summary);
    }

    Ok(())
}

async fn handle_test(ollama_client: &OllamaClient) -> anyhow::Result<()> {
    info!("Ollama 연결 테스트 중...");

    match ollama_client.check_connection().await {
        Ok(true) => {
            println!("✓ Ollama 연결 성공!");

            // 간단한 테스트 쿼리
            info!("테스트 쿼리 전송 중...");
            match ollama_client.generate("안녕하세요. 간단히 인사해주세요.").await {
                Ok(response) => {
                    println!("\n테스트 응답:\n{}", response);
                    println!("\n✓ Ollama가 정상적으로 작동합니다!");
                }
                Err(e) => {
                    error!("테스트 쿼리 실패: {}", e);
                    println!("✗ Ollama 응답에 문제가 있습니다: {}", e);
                }
            }
        }
        Ok(false) => {
            println!("✗ Ollama에 연결할 수 없습니다.");
            println!("Ollama가 실행 중인지 확인하세요: ollama serve");
        }
        Err(e) => {
            error!("연결 테스트 실패: {}", e);
            println!("✗ Ollama 연결 실패: {}", e);
        }
    }

    Ok(())
}
