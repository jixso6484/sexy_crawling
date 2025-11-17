use sexy_crawling::utils::{BrowserProfile, BrowserType, AdvancedAntiBotConfig, CrawlSessionState};
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    println!("🔬 고급 봇 탐지 회피 시스템 테스트\n");

    // 1. 브라우저 프로필 테스트
    println!("📱 브라우저 프로필 생성:");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    let chrome = BrowserProfile::random_chrome();
    println!("🌐 Chrome 프로필:");
    println!("  User-Agent: {}", chrome.user_agent);
    println!("  Platform: {}", chrome.platform);
    println!("  Language: {}", chrome.accept_language);
    println!("  Viewport: {}x{}\n", chrome.viewport_width, chrome.viewport_height);

    let firefox = BrowserProfile::random_firefox();
    println!("🦊 Firefox 프로필:");
    println!("  User-Agent: {}", firefox.user_agent);
    println!("  Platform: {}", firefox.platform);
    println!("  Language: {}\n", firefox.accept_language);

    let safari = BrowserProfile::random_safari();
    println!("🧭 Safari 프로필:");
    println!("  User-Agent: {}", safari.user_agent);
    println!("  Platform: {}\n", safari.platform);

    // 2. 헤더 생성 테스트
    println!("\n📋 헤더 생성 테스트:");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    let headers = chrome.build_headers(Some("https://www.aliexpress.com/"));
    println!("Chrome 헤더 ({} 개):", headers.len());
    for (key, value) in headers.iter() {
        if let Ok(v) = value.to_str() {
            println!("  {}: {}", key, v);
        }
    }

    // 3. 세션 상태 테스트
    println!("\n\n🔄 세션 상태 관리 테스트:");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    let mut session = CrawlSessionState::new();
    println!("새 세션 생성:");
    println!("  브라우저: {:?}", session.profile.browser_type);
    println!("  User-Agent: {}", session.profile.user_agent);

    println!("\n페이지 방문 시뮬레이션:");
    let pages = vec![
        "https://www.aliexpress.com/",
        "https://www.aliexpress.com/category/electronics",
        "https://www.aliexpress.com/item/12345.html",
    ];

    for page in pages {
        session.visit_url(page.to_string());
        println!("  방문: {}", page);
        if let Some(ref referer) = session.current_referer {
            println!("    Referer: {}", referer);
        }
    }

    println!("\n세션 통계:");
    println!("  방문한 페이지: {}개", session.visited_urls.len());
    println!("  세션 지속 시간: {:?}", session.session_duration());

    // 4. 지능형 딜레이 테스트
    println!("\n\n⏱️  지능형 딜레이 테스트 (가우시안 분포):");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    let config = AdvancedAntiBotConfig::default();
    println!("설정:");
    println!("  평균 딜레이: {:.1}초", (config.min_delay_secs + config.max_delay_secs) / 2.0);
    println!("  범위: {:.1}~{:.1}초", config.min_delay_secs, config.max_delay_secs);
    println!("  표준편차: {:.1}\n", config.std_dev);

    println!("10번의 딜레이 측정:");
    let mut delays = Vec::new();
    for i in 1..=10 {
        let start = tokio::time::Instant::now();
        config.intelligent_delay().await;
        let elapsed = start.elapsed().as_secs_f64();
        delays.push(elapsed);
        println!("  #{}: {:.3}초", i, elapsed);
    }

    let avg: f64 = delays.iter().sum::<f64>() / delays.len() as f64;
    let variance: f64 = delays.iter()
        .map(|d| (d - avg).powi(2))
        .sum::<f64>() / delays.len() as f64;
    let std_dev = variance.sqrt();

    println!("\n통계:");
    println!("  평균: {:.3}초", avg);
    println!("  표준편차: {:.3}초", std_dev);
    println!("  최소: {:.3}초", delays.iter().fold(f64::INFINITY, |a, &b| a.min(b)));
    println!("  최대: {:.3}초", delays.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b)));

    // 5. 프로필 분포 테스트
    println!("\n\n📊 브라우저 프로필 분포 (100개 샘플):");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    let mut chrome_count = 0;
    let mut firefox_count = 0;
    let mut safari_count = 0;

    for _ in 0..100 {
        let profile = BrowserProfile::random();
        match profile.browser_type {
            BrowserType::Chrome => chrome_count += 1,
            BrowserType::Firefox => firefox_count += 1,
            BrowserType::Safari => safari_count += 1,
            BrowserType::Edge => chrome_count += 1,  // Edge는 Chrome과 같이 카운트
        }
    }

    println!("  Chrome: {}% (목표: 70%)", chrome_count);
    println!("  Firefox: {}% (목표: 20%)", firefox_count);
    println!("  Safari: {}% (목표: 10%)", safari_count);

    println!("\n\n✅ 모든 테스트 완료!");
    println!("\n💡 주요 기능:");
    println!("  ✅ 브라우저 프로필 시스템 (Chrome, Firefox, Safari)");
    println!("  ✅ 일관된 헤더 생성 (브라우저별 특성 반영)");
    println!("  ✅ 세션 상태 관리 (Referer 체인)");
    println!("  ✅ 가우시안 분포 기반 지능형 딜레이");
    println!("  ✅ 통계적으로 자연스러운 행동 패턴");

    Ok(())
}
