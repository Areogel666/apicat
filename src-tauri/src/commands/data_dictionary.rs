use crate::{db::AppDb, error::CmdResult, types::{DataDictionary, DictionaryItem}};
use tauri::State;
use sqlx::Sqlite;

const DICT_COLS: &str = "id, code, name, description, builtin, project_id, created_at, updated_at";
const ITEM_COLS: &str = "id, dictionary_id, label, value, description, sort_order";

/// 获取数据字典列表（全局共享 builtin + 指定项目自定义）
#[tauri::command]
pub async fn list_dictionaries(
    db: State<'_, AppDb>,
    project_id: Option<i64>,
) -> CmdResult<Vec<DataDictionary>> {
    let sql = format!(
        "SELECT {DICT_COLS} FROM data_dictionaries \
         WHERE project_id IS NULL OR project_id = ?1 \
         ORDER BY builtin DESC, id ASC"
    );
    let rows = sqlx::query_as::<Sqlite, DataDictionary>(&sql)
        .bind(project_id.unwrap_or(0))
        .fetch_all(&db.0)
        .await?;
    Ok(rows)
}

/// 创建数据字典（默认项目无关 = 全局共享）
#[tauri::command]
pub async fn create_dictionary(
    db: State<'_, AppDb>,
    code: String,
    name: String,
    description: Option<String>,
    project_id: Option<i64>,
) -> CmdResult<DataDictionary> {
    let sql = format!(
        "INSERT INTO data_dictionaries (code, name, description, builtin, project_id) \
         VALUES (?1, ?2, ?3, 0, ?4) RETURNING {DICT_COLS}"
    );
    let row = sqlx::query_as::<Sqlite, DataDictionary>(&sql)
        .bind(&code)
        .bind(&name)
        .bind(description.unwrap_or_default())
        .bind(project_id)
        .fetch_one(&db.0)
        .await?;
    Ok(row)
}

/// 更新数据字典名称 / 描述
#[tauri::command]
pub async fn update_dictionary(
    db: State<'_, AppDb>,
    id: i64,
    name: Option<String>,
    description: Option<String>,
) -> CmdResult<DataDictionary> {
    sqlx::query(
        "UPDATE data_dictionaries SET \
         name = COALESCE(?1, name), \
         description = COALESCE(?2, description), \
         updated_at = CURRENT_TIMESTAMP \
         WHERE id = ?3",
    )
    .bind(&name)
    .bind(&description)
    .bind(id)
    .execute(&db.0)
    .await?;
    let sql = format!("SELECT {DICT_COLS} FROM data_dictionaries WHERE id = ?1");
    let row = sqlx::query_as::<Sqlite, DataDictionary>(&sql).bind(id).fetch_one(&db.0).await?;
    Ok(row)
}

/// 删除数据字典（字典项由 FK ON DELETE CASCADE 清理）
#[tauri::command]
pub async fn delete_dictionary(db: State<'_, AppDb>, id: i64) -> CmdResult<()> {
    sqlx::query("DELETE FROM data_dictionaries WHERE id = ?1")
        .bind(id)
        .execute(&db.0)
        .await?;
    Ok(())
}

/// 获取某字典的全部字典项
#[tauri::command]
pub async fn list_dictionary_items(
    db: State<'_, AppDb>,
    dictionary_id: i64,
) -> CmdResult<Vec<DictionaryItem>> {
    let sql = format!(
        "SELECT {ITEM_COLS} FROM dictionary_items WHERE dictionary_id = ?1 ORDER BY sort_order ASC, id ASC"
    );
    let rows = sqlx::query_as::<Sqlite, DictionaryItem>(&sql)
        .bind(dictionary_id)
        .fetch_all(&db.0)
        .await?;
    Ok(rows)
}

/// 创建字典项
#[tauri::command]
pub async fn create_dictionary_item(
    db: State<'_, AppDb>,
    dictionary_id: i64,
    label: String,
    value: String,
    description: Option<String>,
) -> CmdResult<DictionaryItem> {
    let sql = format!(
        "INSERT INTO dictionary_items (dictionary_id, label, value, description, sort_order) \
         VALUES (?1, ?2, ?3, ?4, 0) RETURNING {ITEM_COLS}"
    );
    let row = sqlx::query_as::<Sqlite, DictionaryItem>(&sql)
        .bind(dictionary_id)
        .bind(&label)
        .bind(&value)
        .bind(description.unwrap_or_default())
        .fetch_one(&db.0)
        .await?;
    Ok(row)
}

/// 更新字典项
#[tauri::command]
pub async fn update_dictionary_item(
    db: State<'_, AppDb>,
    id: i64,
    label: Option<String>,
    value: Option<String>,
    description: Option<String>,
    sort_order: Option<i64>,
) -> CmdResult<DictionaryItem> {
    sqlx::query(
        "UPDATE dictionary_items SET \
         label = COALESCE(?1, label), \
         value = COALESCE(?2, value), \
         description = COALESCE(?3, description), \
         sort_order = COALESCE(?4, sort_order) \
         WHERE id = ?5",
    )
    .bind(&label)
    .bind(&value)
    .bind(&description)
    .bind(sort_order)
    .bind(id)
    .execute(&db.0)
    .await?;
    let sql = format!("SELECT {ITEM_COLS} FROM dictionary_items WHERE id = ?1");
    let row = sqlx::query_as::<Sqlite, DictionaryItem>(&sql).bind(id).fetch_one(&db.0).await?;
    Ok(row)
}

/// 删除字典项
#[tauri::command]
pub async fn delete_dictionary_item(db: State<'_, AppDb>, id: i64) -> CmdResult<()> {
    sqlx::query("DELETE FROM dictionary_items WHERE id = ?1")
        .bind(id)
        .execute(&db.0)
        .await?;
    Ok(())
}