//! 项目级公共 Headers 模板（按项目落库）
//!
//! 编辑粒度是「整份列表」：前端编辑期做防抖，一次请求提交完整数组，
//! 后端事务内全删全插。模板条目通常只有几条，逐条 diff 没有收益。

use crate::{db::AppDb, error::CmdResult, sql_cols::HEADER_TEMPLATE_COLS, types::HeaderTemplate};
use serde::Deserialize;
use tauri::State;

/// 保存时的输入形状：id / 排序由后端生成，前端不需要关心
#[derive(Debug, Deserialize)]
pub struct HeaderTemplateInput {
    pub key: String,
    pub value: String,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    #[serde(default)]
    pub description: String,
}

fn default_enabled() -> bool {
    true
}

/// 查询项目下的模板（按列表顺序）
pub async fn list_header_templates_impl(
    pool: &sqlx::SqlitePool,
    project_id: i64,
) -> Result<Vec<HeaderTemplate>, crate::error::AppError> {
    let rows = sqlx::query_as::<_, HeaderTemplate>(&format!(
        "SELECT {HEADER_TEMPLATE_COLS} FROM header_templates WHERE project_id=? \
         ORDER BY sort_order, id"
    ))
    .bind(project_id)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// 整批替换项目下的模板（事务保证不会出现「删了没插」的中间态）
pub async fn save_header_templates_impl(
    pool: &sqlx::SqlitePool,
    project_id: i64,
    items: &[HeaderTemplateInput],
) -> Result<Vec<HeaderTemplate>, crate::error::AppError> {
    let mut tx = pool.begin().await?;

    sqlx::query("DELETE FROM header_templates WHERE project_id=?")
        .bind(project_id)
        .execute(&mut *tx)
        .await?;

    for (idx, item) in items.iter().enumerate() {
        sqlx::query(
            "INSERT INTO header_templates (project_id, key, value, enabled, description, sort_order) \
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(project_id)
        .bind(&item.key)
        .bind(&item.value)
        .bind(item.enabled)
        .bind(&item.description)
        .bind(idx as i64)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;
    list_header_templates_impl(pool, project_id).await
}

/// 查询项目下的公共 Headers 模板
#[tauri::command]
pub async fn list_header_templates(
    db: State<'_, AppDb>,
    project_id: i64,
) -> CmdResult<Vec<HeaderTemplate>> {
    list_header_templates_impl(&db.0, project_id).await
}

/// 整批保存项目下的公共 Headers 模板
#[tauri::command]
pub async fn save_header_templates(
    db: State<'_, AppDb>,
    project_id: i64,
    items: Vec<HeaderTemplateInput>,
) -> CmdResult<Vec<HeaderTemplate>> {
    save_header_templates_impl(&db.0, project_id, &items).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

    /// 内存库每个连接独立，必须限 1 连接，否则建表与读写落到不同库
    async fn mem_pool() -> sqlx::SqlitePool {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        sqlx::query("CREATE TABLE projects (id INTEGER PRIMARY KEY)")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO projects (id) VALUES (1), (2)")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::raw_sql(include_str!("../../migrations/0004_header_templates.sql"))
            .execute(&pool)
            .await
            .unwrap();
        pool
    }

    fn item(key: &str, enabled: bool) -> HeaderTemplateInput {
        HeaderTemplateInput {
            key: key.to_string(),
            value: format!("v-{key}"),
            enabled,
            description: String::new(),
        }
    }

    /// 锁住两条语义：项目间隔离 + 保存是全量替换（不是追加）
    #[tokio::test]
    async fn templates_are_project_scoped_and_replaced() {
        let pool = mem_pool().await;

        save_header_templates_impl(&pool, 1, &[item("A", true), item("B", false)])
            .await
            .unwrap();
        save_header_templates_impl(&pool, 2, &[item("C", true)])
            .await
            .unwrap();

        let p1 = list_header_templates_impl(&pool, 1).await.unwrap();
        assert_eq!(p1.len(), 2);
        assert_eq!(p1[0].key, "A");
        assert!(p1[0].enabled);
        assert!(!p1[1].enabled);

        let p2 = list_header_templates_impl(&pool, 2).await.unwrap();
        assert_eq!(p2.len(), 1);
        assert_eq!(p2[0].key, "C");

        // 全量替换：旧条目全部消失
        save_header_templates_impl(&pool, 1, &[item("D", true)])
            .await
            .unwrap();
        let p1 = list_header_templates_impl(&pool, 1).await.unwrap();
        assert_eq!(p1.len(), 1);
        assert_eq!(p1[0].key, "D");

        // 清空项目 1 不影响项目 2
        save_header_templates_impl(&pool, 1, &[]).await.unwrap();
        assert!(list_header_templates_impl(&pool, 1).await.unwrap().is_empty());
        assert_eq!(list_header_templates_impl(&pool, 2).await.unwrap().len(), 1);
    }
}
