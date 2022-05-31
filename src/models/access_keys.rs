use std::str::FromStr;

use bigdecimal::BigDecimal;
use sqlx::Arguments;

use crate::models::FieldCount;

#[derive(Debug, sqlx::FromRow, FieldCount)]
pub struct AccessKey {
    pub public_key: String,
    pub account_id: String,
    pub created_by_receipt_id: Option<String>,
    pub deleted_by_receipt_id: Option<String>,
    pub permission_kind: String,
}

impl crate::models::MySqlMethods for AccessKey {
    fn add_to_args(&self, args: &mut sqlx::postgres::PgArguments) {
        args.add(&self.public_key);
        args.add(&self.account_id);
        args.add(&self.created_by_receipt_id);
        args.add(&self.deleted_by_receipt_id);
        args.add(&self.permission_kind);
    }

    fn insert_query(items_count: usize) -> anyhow::Result<String> {
        Ok("INSERT INTO access_keys VALUES ".to_owned()
            + &crate::models::create_placeholders(items_count, AccessKey::field_count())?
            + " ON CONFLICT DO NOTHING")
    }

    fn name() -> String {
        "access_keys".to_string()
    }
}
