use sqlx::{MySql, Transaction};

pub async fn upsert_environment(
    tx: &mut Transaction<'_, MySql>,
    environment: &str,
    namespace: &str,
) -> anyhow::Result<u64> {
    Ok(sqlx::query!(
            r#"
                INSERT INTO environments (name, namespace) VALUES (?, ?) ON DUPLICATE KEY UPDATE name = VALUES(name)
            "#,
            environment,
            namespace
        )
        .execute(&mut **tx)
        .await?
        .last_insert_id())
}

pub async fn get_env_id(
    tx: &mut Transaction<'_, MySql>,
    environment: &str,
) -> anyhow::Result<Option<u64>> {
    let env_id = sqlx::query!(
        r#"
                SELECT id AS `id: u64` FROM environments WHERE name = ?
        "#,
        environment
    )
    .fetch_one(&mut **tx)
    .await?;

    Ok(env_id.id)
}

pub async fn add_section(tx: &mut Transaction<'_, MySql>, section: &str) -> anyhow::Result<u64> {
    Ok(sqlx::query!(
        r#"
                INSERT INTO config_sections (name) VALUES (?) ON DUPLICATE KEY UPDATE id = id
        "#,
        section
    )
    .execute(&mut **tx)
    .await?
    .last_insert_id())
}

pub async fn delete_section(tx: &mut Transaction<'_, MySql>, section: &str) -> anyhow::Result<u64> {
    Ok(sqlx::query!(
        r#"
                DELETE FROM config_sections WHERE name = ?
        "#,
        section
    )
    .execute(&mut **tx)
    .await?
    .last_insert_id())
}

pub async fn list_sections(tx: &mut Transaction<'_, MySql>) -> anyhow::Result<Vec<String>> {
    let sections = sqlx::query!(
        r#"
                SELECT name FROM config_sections
        "#
    )
    .fetch_all(&mut **tx)
    .await?;

    Ok(sections.into_iter().filter_map(|s| s.name).collect())
}

pub async fn set_config_value(
    tx: &mut Transaction<'_, MySql>,
    section: &str,
    key: &str,
    value: &str,
    value_type: &str,
) -> anyhow::Result<u64> {
    Ok(sqlx::query!(
            r#"
                INSERT INTO config_values
                    (section_id, cfg_key, cfg_value, value_type_id, default_id) 
                VALUES (
                    (SELECT id FROM config_sections WHERE name = ?),
                    ?,
                    ?,
                    (SELECT id FROM config_value_types WHERE name = ?),
                    (SELECT id FROM config_defaults WHERE cfg_key = VALUES(cfg_key) AND section_id = VALUES(section_id))
                )
            "#,
            section,
            key,
            value,
            value_type
        )
        .execute(&mut **tx)
        .await?
        .last_insert_id())
}

pub async fn add_env_cfg_value(
    tx: &mut Transaction<'_, MySql>,
    env_id: u64,
    cfg_id: u64,
) -> anyhow::Result<u64> {
    Ok(sqlx::query!(
            r#"
                INSERT INTO environments_config_values (environment_id, config_value_id) VALUES (?, ?)
            "#,
            env_id,
            cfg_id
        )
        .execute(&mut **tx)
        .await?
        .last_insert_id())
}
