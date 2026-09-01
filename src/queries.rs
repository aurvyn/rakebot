use crate::{BaseItem, Item, Modifier, Passive};
use sqlx::SqlitePool;

type ItemId = u32;
type PassiveId = u32;
pub type QualityId = u8;
type ModifierId = u8;

pub async fn try_create_tables(pool: &SqlitePool) {
    sqlx::query(
        "
        CREATE TABLE IF NOT EXISTS user (
            id         INTEGER PRIMARY KEY,
            exp        INTEGER NOT NULL DEFAULT 0,
            leaves     INTEGER NOT NULL DEFAULT 0,
            last_raked INTEGER NOT NULL DEFAULT 0,
            last_daily INTEGER NOT NULL DEFAULT 0
        )",
    )
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(
        "
        CREATE TABLE IF NOT EXISTS item (
            user_id  INTEGER NOT NULL,
            item_id  INTEGER NOT NULL,
            quantity INTEGER NOT NULL DEFAULT 0,
            quality  INTEGER NOT NULL DEFAULT 0,
            modifier INTEGER NOT NULL DEFAULT 0,
            PRIMARY KEY (user_id, item_id, quality, modifier),
            FOREIGN KEY (user_id) REFERENCES user(id)
        )",
    )
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(
        "
        CREATE TABLE IF NOT EXISTS passive (
            user_id    INTEGER NOT NULL,
            passive_id INTEGER NOT NULL,
            expires_at INTEGER NOT NULL DEFAULT 0,
            PRIMARY KEY (user_id, passive_id),
            FOREIGN KEY (user_id) REFERENCES user(id)
        )",
    )
    .execute(pool)
    .await
    .unwrap();
}

pub async fn try_register(user_id: i64, pool: &SqlitePool) {
    sqlx::query(&format!(
        "INSERT OR IGNORE INTO user (id) VALUES ({user_id})"
    ))
    .execute(pool)
    .await
    .unwrap();
}

pub async fn get_from_user(field: &str, user_id: i64, pool: &SqlitePool) -> i64 {
    let (result,) = sqlx::query_as(&format!("SELECT {field} FROM user WHERE id = {user_id}"))
        .fetch_one(pool)
        .await
        .unwrap();
    result
}

pub async fn get_passives(user_id: i64, time: i64, pool: &SqlitePool) -> Vec<(PassiveId, i64)> {
    sqlx::query_as(&format!(
        "SELECT passive_id, expires_at FROM passive WHERE user_id = {user_id} AND expires_at > {time}"
    ))
    .fetch_all(pool)
    .await
    .unwrap()
}

/// Returns the amount that the user owns.
pub async fn get_item(
    user_id: i64,
    item_id: ItemId,
    quality: QualityId,
    modifier: ModifierId,
    pool: &SqlitePool,
) -> Option<u32> {
    sqlx::query_as(&format!(
        "SELECT quantity FROM item WHERE user_id = {user_id} AND item_id = {item_id} AND quality = {quality} AND modifier = {modifier} AND quantity > 0"
    ))
    .fetch_one(pool)
    .await
    .ok()
    .map(|(q,)| q)
}

pub async fn get_items(user_id: i64, pool: &SqlitePool) -> Vec<(Item, u32)> {
    sqlx::query_as(&format!(
        "SELECT item_id, quantity, quality, modifier FROM item WHERE user_id = {user_id} AND quantity > 0"
    ))
    .fetch_all(pool)
    .await
    .unwrap()
    .iter()
    .map(|(item_id, quantity, quality, modifier)| {
        let item_id: ItemId = *item_id;
        let modifier_id: ModifierId = *modifier;
        (
            Item {
                base: BaseItem::from_repr(item_id as usize).unwrap(),
                quality: *quality,
                modifier: Modifier::from_repr(modifier_id as usize).unwrap(),
            },
            *quantity,
        )
    })
    .collect()
}

pub async fn get_lb(pool: &SqlitePool, server_ids: Vec<u64>, limit: Option<u8>) -> Vec<(u64, i64)> {
    sqlx::query_as(&format!(
        "SELECT id, exp FROM user {} ORDER BY exp DESC, leaves DESC {}",
        if server_ids.is_empty() {
            String::new()
        } else {
            format!(
                "WHERE id IN ({})",
                server_ids
                    .iter()
                    .map(|id| id.to_string())
                    .collect::<Vec<_>>()
                    .join(",")
            )
        },
        if let Some(lim) = limit {
            format!("LIMIT {lim}")
        } else {
            String::new()
        }
    ))
    .fetch_all(pool)
    .await
    .unwrap()
}

pub async fn update_raking(
    user_id: i64,
    exp: i32,
    leaves: i32,
    field: &str,
    last_raked: i64,
    pool: &SqlitePool,
) {
    sqlx::query(&format!(
        "UPDATE user SET exp = exp + {exp}, leaves = leaves + {leaves}, {field} = {last_raked} WHERE id = {user_id}"
    ))
    .execute(pool)
    .await
    .unwrap();
}

pub async fn add_item(user_id: i64, item: Item, pool: &SqlitePool) {
    let item_id = item.base as ItemId;
    sqlx::query(&format!(
        "INSERT OR IGNORE INTO item (user_id, item_id, quantity) VALUES ({user_id}, {item_id}, 0)",
    ))
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(&format!(
        "UPDATE item SET quantity = quantity + 1 WHERE user_id = {user_id} AND item_id = {item_id}",
    ))
    .execute(pool)
    .await
    .unwrap();
}

pub async fn add_passive(user_id: i64, expires_at: i64, passive: Passive, pool: &SqlitePool) {
    let passive_id = passive as i32;
    sqlx::query(&format!(
        "INSERT OR IGNORE INTO passive (user_id, passive_id, expires_at) VALUES ({user_id}, {passive_id}, 0)"
    ))
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(&format!(
        "UPDATE passive SET expires_at = {expires_at} WHERE user_id = {user_id} AND passive_id = {passive_id}"
    ))
    .execute(pool)
    .await
    .unwrap();
}

pub async fn sell_item(user_id: i64, amount: i32, item: Item, pool: &SqlitePool) {
    let item_id = item.base as ItemId;
    sqlx::query(&format!(
        "UPDATE item SET quantity = quantity - {amount} WHERE user_id = {user_id} AND item_id = {item_id}",
    ))
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(&format!(
        "UPDATE user SET leaves = leaves + {} WHERE id = {user_id}",
        amount * item.base.selling_price()
    ))
    .execute(pool)
    .await
    .unwrap();
}
