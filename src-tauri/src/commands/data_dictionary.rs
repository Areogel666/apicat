use crate::{db::AppDb, error::CmdResult, types::{DataDictionary, DictionaryItem, FieldDictionaryRule, FieldDictionaryOverride}};
use tauri::State;
use sqlx::Sqlite;

const DICT_COLS: &str = "id, code, name, description, builtin, project_id, created_at, updated_at";
const ITEM_COLS: &str = "id, dictionary_id, label, value, description, sort_order";

/// 批量写入用字典项输入（1.0.4 fix：一键新增 / JSON 预览编辑）
#[derive(serde::Deserialize)]
pub struct DictionaryItemInput {
    pub label: String,
    pub value: String,
    pub description: Option<String>,
}

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

/// 一键新建字典 + 初始字典项（同一事务，1.0.4 fix）
#[tauri::command]
pub async fn create_dictionary_with_items(
    db: State<'_, AppDb>,
    code: String,
    name: String,
    description: Option<String>,
    project_id: Option<i64>,
    items: Vec<DictionaryItemInput>,
) -> CmdResult<DataDictionary> {
    let mut tx = db.0.begin().await?;
    let dict_sql = format!(
        "INSERT INTO data_dictionaries (code, name, description, builtin, project_id) \
         VALUES (?1, ?2, ?3, 0, ?4) RETURNING {DICT_COLS}"
    );
    let dict: DataDictionary = sqlx::query_as::<Sqlite, DataDictionary>(&dict_sql)
        .bind(&code)
        .bind(&name)
        .bind(description.unwrap_or_default())
        .bind(project_id)
        .fetch_one(&mut *tx)
        .await?;
    for (i, item) in items.iter().enumerate() {
        sqlx::query(
            "INSERT INTO dictionary_items (dictionary_id, label, value, description, sort_order) \
             VALUES (?1, ?2, ?3, ?4, ?5)",
        )
        .bind(dict.id)
        .bind(&item.label)
        .bind(&item.value)
        .bind(item.description.clone().unwrap_or_default())
        .bind(i as i64)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    Ok(dict)
}

// ═══════════════════════════════════════════════════════════
//  1.0.4：字典「字段名绑定」—— 规则 / 例外 / 跨项目复制
// ═══════════════════════════════════════════════════════════

/// 项目字段绑定规则列表（field_name ↔ dictionary_id，一对一）
#[tauri::command]
pub async fn list_field_rules(
    db: State<'_, AppDb>,
    project_id: i64,
) -> CmdResult<Vec<FieldDictionaryRule>> {
    let rows = sqlx::query_as::<Sqlite, FieldDictionaryRule>(
        "SELECT id, project_id, field_name, dictionary_id FROM field_dictionary_rules \
         WHERE project_id = ?1 ORDER BY field_name",
    )
    .bind(project_id)
    .fetch_all(&db.0)
    .await?;
    Ok(rows)
}

/// 新增 / 更新 项目字段绑定规则（dictionary_id 必填；同字段 upsert）
#[tauri::command]
pub async fn set_field_rule(
    db: State<'_, AppDb>,
    project_id: i64,
    field_name: String,
    dictionary_id: i64,
) -> CmdResult<()> {
    sqlx::query(
        "INSERT INTO field_dictionary_rules (project_id, field_name, dictionary_id) \
         VALUES (?1, ?2, ?3) \
         ON CONFLICT(project_id, field_name) DO UPDATE SET dictionary_id = ?3",
    )
    .bind(project_id)
    .bind(&field_name)
    .bind(dictionary_id)
    .execute(&db.0)
    .await?;
    Ok(())
}

/// 删除项目字段绑定规则
#[tauri::command]
pub async fn delete_field_rule(
    db: State<'_, AppDb>,
    project_id: i64,
    field_name: String,
) -> CmdResult<()> {
    sqlx::query("DELETE FROM field_dictionary_rules WHERE project_id = ?1 AND field_name = ?2")
        .bind(project_id)
        .bind(&field_name)
        .execute(&db.0)
        .await?;
    Ok(())
}

/// 接口×字段名的例外列表（换绑/解绑，优先于规则）
#[tauri::command]
pub async fn list_field_overrides(
    db: State<'_, AppDb>,
    project_id: i64,
) -> CmdResult<Vec<FieldDictionaryOverride>> {
    let rows = sqlx::query_as::<Sqlite, FieldDictionaryOverride>(
        "SELECT id, project_id, request_id, field_name, dictionary_id \
         FROM field_dictionary_overrides WHERE project_id = ?1",
    )
    .bind(project_id)
    .fetch_all(&db.0)
    .await?;
    Ok(rows)
}

/// 设置接口×字段名的例外（dictionary_id = null 表示解除绑定；同(request_id,field) upsert）
#[tauri::command]
pub async fn set_field_override(
    db: State<'_, AppDb>,
    project_id: i64,
    request_id: i64,
    field_name: String,
    dictionary_id: Option<i64>,
) -> CmdResult<()> {
    sqlx::query(
        "INSERT INTO field_dictionary_overrides (project_id, request_id, field_name, dictionary_id) \
         VALUES (?1, ?2, ?3, ?4) \
         ON CONFLICT(project_id, request_id, field_name) DO UPDATE SET dictionary_id = ?4",
    )
    .bind(project_id)
    .bind(request_id)
    .bind(&field_name)
    .bind(dictionary_id)
    .execute(&db.0)
    .await?;
    Ok(())
}

/// 删除接口×字段名的例外（恢复跟随项目规则）
#[tauri::command]
pub async fn delete_field_override(
    db: State<'_, AppDb>,
    project_id: i64,
    request_id: i64,
    field_name: String,
) -> CmdResult<()> {
    sqlx::query(
        "DELETE FROM field_dictionary_overrides \
         WHERE project_id = ?1 AND request_id = ?2 AND field_name = ?3",
    )
    .bind(project_id)
    .bind(request_id)
    .bind(&field_name)
    .execute(&db.0)
    .await?;
    Ok(())
}

/// 1.0.4：判断项目内某字段是否已有绑定规则（供跨项目复制/导入复用）
pub(crate) async fn field_rule_exists(
    conn: &mut sqlx::SqliteConnection,
    project_id: i64,
    field_name: &str,
) -> CmdResult<bool> {
    let r: Option<(i64,)> = sqlx::query_as(
        "SELECT id FROM field_dictionary_rules WHERE project_id = ?1 AND field_name = ?2",
    )
    .bind(project_id)
    .bind(field_name)
    .fetch_optional(&mut *conn)
    .await?;
    Ok(r.is_some())
}

/// 把字典（含全部字典项 + 源项目里绑定它的字段规则）复制到目标项目。
/// 目标项目已有同 code 字典 → 报错；目标项目已有同字段名的规则 → 跳过该字段。
#[tauri::command]
pub async fn copy_dictionary_to_project(
    db: State<'_, AppDb>,
    source_dictionary_id: i64,
    source_project_id: i64,
    target_project_id: i64,
) -> CmdResult<DataDictionary> {
    let src = sqlx::query_as::<Sqlite, DataDictionary>(
        format!("SELECT {DICT_COLS} FROM data_dictionaries WHERE id = ?1").as_str(),
    )
    .bind(source_dictionary_id)
    .fetch_one(&db.0)
    .await
    .map_err(|_| crate::error::AppError::Custom("源字典不存在".to_string()))?;

    // code 列有全局 UNIQUE 约束，检查范围必须是全局而非仅目标项目
    let exists: Option<(i64,)> = sqlx::query_as(
        "SELECT id FROM data_dictionaries WHERE code = ?1",
    )
    .bind(&src.code)
    .fetch_optional(&db.0)
    .await?;
    if exists.is_some() {
        return Err(crate::error::AppError::Custom(
            format!("字典 code「{}」已被占用（全局唯一），无法复制。请先重命名源字典或目标字典。", src.code),
        ));
    }

    let mut tx = db.0.begin().await?;

    // 1. 复制字典本体（target 维度）
    let dict_sql = format!(
        "INSERT INTO data_dictionaries (code, name, description, builtin, project_id) \
         VALUES (?1, ?2, ?3, ?4, ?5) RETURNING {DICT_COLS}"
    );
    let new_dict: DataDictionary = sqlx::query_as::<Sqlite, DataDictionary>(&dict_sql)
        .bind(&src.code)
        .bind(&src.name)
        .bind(&src.description)
        .bind(0)
        .bind(target_project_id)
        .fetch_one(&mut *tx)
        .await?;

    // 2. 复制全部字典项
    let items: Vec<DictionaryItem> =
        sqlx::query_as::<Sqlite, DictionaryItem>(
            "SELECT id, dictionary_id, label, value, description, sort_order \
             FROM dictionary_items WHERE dictionary_id = ?1 ORDER BY sort_order, id",
        )
        .bind(source_dictionary_id)
        .fetch_all(&db.0)
        .await?;
    for item in &items {
        sqlx::query(
            "INSERT INTO dictionary_items (dictionary_id, label, value, description, sort_order) \
             VALUES (?1, ?2, ?3, ?4, ?5)",
        )
        .bind(new_dict.id)
        .bind(&item.label)
        .bind(&item.value)
        .bind(&item.description)
        .bind(item.sort_order)
        .execute(&mut *tx)
        .await?;
    }

    // 3. 复制「源项目里绑定该字典」的字段规则到目标项目（同字段名已存在则跳过）
    let rules: Vec<FieldDictionaryRule> = sqlx::query_as::<Sqlite, FieldDictionaryRule>(
        "SELECT id, project_id, field_name, dictionary_id FROM field_dictionary_rules \
         WHERE project_id = ?1 AND dictionary_id = ?2",
    )
    .bind(source_project_id)
    .bind(source_dictionary_id)
    .fetch_all(&db.0)
    .await?;
    for rule in &rules {
        if !field_rule_exists(&mut *tx, target_project_id, &rule.field_name).await? {
            sqlx::query(
                "INSERT INTO field_dictionary_rules (project_id, field_name, dictionary_id) \
                 VALUES (?1, ?2, ?3)",
            )
            .bind(target_project_id)
            .bind(&rule.field_name)
            .bind(new_dict.id)
            .execute(&mut *tx)
            .await?;
        }
    }

    tx.commit().await?;
    Ok(new_dict)
}

/// 以 JSON 全量替换某字典的字典项（先清空再插入，同一事务，1.0.4 fix）。
/// 返回替换后的字典项列表。
#[tauri::command]
pub async fn replace_dictionary_items(
    db: State<'_, AppDb>,
    dictionary_id: i64,
    items: Vec<DictionaryItemInput>,
) -> CmdResult<Vec<DictionaryItem>> {
    let mut tx = db.0.begin().await?;
    sqlx::query("DELETE FROM dictionary_items WHERE dictionary_id = ?1")
        .bind(dictionary_id)
        .execute(&mut *tx)
        .await?;
    for (i, item) in items.iter().enumerate() {
        sqlx::query(
            "INSERT INTO dictionary_items (dictionary_id, label, value, description, sort_order) \
             VALUES (?1, ?2, ?3, ?4, ?5)",
        )
        .bind(dictionary_id)
        .bind(&item.label)
        .bind(&item.value)
        .bind(item.description.clone().unwrap_or_default())
        .bind(i as i64)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    let sql = format!(
        "SELECT {ITEM_COLS} FROM dictionary_items WHERE dictionary_id = ?1 ORDER BY sort_order ASC, id ASC"
    );
    let rows = sqlx::query_as::<Sqlite, DictionaryItem>(&sql)
        .bind(dictionary_id)
        .fetch_all(&db.0)
        .await?;
    Ok(rows)
}