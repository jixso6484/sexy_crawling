// 스키마 정의는 schema.sql에 있음
// 이 파일은 스키마 관련 유틸리티 함수를 제공

use anyhow::Result;
use sqlx::SqlitePool;

/// 데이터베이스 초기화 (schema.sql 실행)
pub async fn init_database(pool: &SqlitePool) -> Result<()> {
    let schema_sql = include_str!("../../schema.sql");

    // SQL을 세미콜론으로 분리하여 실행
    for statement in schema_sql.split(';') {
        let trimmed = statement.trim();
        if !trimmed.is_empty() && !trimmed.starts_with("--") {
            sqlx::query(trimmed)
                .execute(pool)
                .await?;
        }
    }

    Ok(())
}
