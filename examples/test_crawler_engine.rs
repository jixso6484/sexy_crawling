use sexy_crawling::{Database, CrawlerEngine, CrawlerEngineConfig};
use sexy_crawling::db::{url_queue::*, sessions::CrawlMode};
use sexy_crawling::utils::HumanBehavior;
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    // 로깅 초기화
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();

    println!("🚀 알리익스프레스 크롤링 시스템 테스트 시작\n");

    // 1. 데이터베이스 초기화
    println!("📊 SQLite 데이터베이스 초기화 중...");
    let db_path = "/home/user/sexy_crawling/test_crawl.db";
    let db = Database::new(db_path).await?;
    println!("✅ 데이터베이스 초기화 완료: {}\n", db_path);

    // 2. 초기 통계 출력
    println!("📈 초기 통계:");
    db.get_stats().await?.print();

    // 3. 크롤링 엔진 설정
    println!("⚙️  크롤링 엔진 설정 중...");
    let config = CrawlerEngineConfig {
        max_concurrent_crawls: 1,
        crawl_mode: CrawlMode::Direct, // 직접 모드 (구글 API 제한 피하기)
        human_behavior: HumanBehavior::fast(), // 빠른 테스트
        save_html: true,
        html_cache_hours: 24,
        max_retries: 3,
        ..Default::default()
    };

    let mut engine = CrawlerEngine::new(db, config);
    println!("✅ 크롤링 엔진 준비 완료!\n");

    // 4. 세션 시작
    println!("🎬 크롤링 세션 시작...");
    let session_id = engine.start_session(Some("gaming mouse")).await?;
    println!("✅ 세션 ID: {}\n", session_id);

    // 5. 알리익스프레스에서 URL 발견 (테스트용으로 2페이지만)
    println!("🔍 알리익스프레스에서 URL 발견 중...");
    match engine.discover_urls_from_aliexpress("gaming mouse", 2).await {
        Ok(count) => {
            println!("✅ {}개의 URL 발견!\n", count);
        }
        Err(e) => {
            println!("⚠️  URL 발견 실패: {}\n", e);
        }
    }

    // 6. 통계 출력
    println!("📊 발견 후 통계:");
    engine.print_stats().await?;

    // 7. 발견된 URL 중 5개만 크롤링 (테스트)
    println!("\n🕷️  상품 크롤링 시작 (최대 5개)...");
    match engine.crawl_pending_urls(5).await {
        Ok(crawled) => {
            println!("✅ {}개 상품 크롤링 완료!\n", crawled);
        }
        Err(e) => {
            println!("⚠️  크롤링 중 오류: {}\n", e);
        }
    }

    // 8. 최종 통계
    println!("📊 최종 통계:");
    engine.print_stats().await?;

    // 9. 세션 종료
    println!("\n🏁 세션 종료 중...");
    engine.finish_session(sexy_crawling::db::sessions::SessionStatus::Completed).await?;
    println!("✅ 세션 종료 완료!");

    println!("\n✨ 테스트 완료! 데이터베이스: {}", db_path);

    Ok(())
}
