use std::collections::HashMap;
use std::convert::TryFrom;

use crate::models;
use anyhow::Context;
use bigdecimal::BigDecimal;
use futures::future::try_join_all;
use futures::try_join;
use tracing::info;

pub(crate) async fn store_accounts(
    pool: &sqlx::Pool<sqlx::Postgres>,
    shards: &[near_indexer_primitives::IndexerShard],
    block_height: near_indexer_primitives::types::BlockHeight,
) -> anyhow::Result<()> {
    let futures = shards.iter().map(|shard| {
        store_accounts_for_chunk(pool, &shard.receipt_execution_outcomes, block_height)
    });

    try_join_all(futures).await.map(|_| ())
}

async fn store_accounts_for_chunk(
    pool: &sqlx::Pool<sqlx::Postgres>,
    outcomes: &[near_indexer_primitives::IndexerExecutionOutcomeWithReceipt],
    block_height: near_indexer_primitives::types::BlockHeight,
) -> anyhow::Result<()> {
    if outcomes.is_empty() {
        return Ok(());
    }
    let successful_receipts = outcomes
        .iter()
        .filter(|outcome_with_receipt| {
            matches!(
                outcome_with_receipt.execution_outcome.outcome.status,
                near_indexer_primitives::views::ExecutionStatusView::SuccessValue(_)
                    | near_indexer_primitives::views::ExecutionStatusView::SuccessReceiptId(_)
            )
        })
        .map(|outcome_with_receipt| &outcome_with_receipt.receipt);

    let mut accounts =
        HashMap::<near_indexer_primitives::types::AccountId, models::accounts::Account>::new();

    for receipt in successful_receipts {
        if let near_indexer_primitives::views::ReceiptEnumView::Action { actions, .. } =
            &receipt.receipt
        {
            for action in actions {
                match action {
                    near_indexer_primitives::views::ActionView::CreateAccount => {
                        accounts.insert(
                            receipt.receiver_id.clone(),
                            models::accounts::Account::new_from_receipt(
                                &receipt.receiver_id,
                                &receipt.receipt_id,
                                block_height,
                            ),
                        );
                    }
                    near_indexer_primitives::views::ActionView::Transfer { .. } => {
                        if receipt.receiver_id.len() == 64usize {
                            let previously_created = models::select_retry_or_panic(pool,
                            "SELECT * FROM accounts WHERE account_id = $1 AND deleted_by_receipt_id IS NULL", &[receipt.receiver_id.to_string()], 10).await?;
                            if previously_created.len() == 0 {
                                accounts.insert(
                                    receipt.receiver_id.clone(),
                                    models::accounts::Account::new_from_receipt(
                                        &receipt.receiver_id,
                                        &receipt.receipt_id,
                                        block_height,
                                    ),
                                );
                            }
                        }
                    }
                    near_indexer_primitives::views::ActionView::DeleteAccount { .. } => {
                        accounts
                            .entry(receipt.receiver_id.clone())
                            .and_modify(|existing_account| {
                                existing_account.deleted_by_receipt_id =
                                    Some(receipt.receipt_id.to_string());
                                existing_account.deleted_by_block_height =
                                    Some(BigDecimal::from(block_height))
                            })
                            .or_insert_with(|| models::accounts::Account {
                                account_id: receipt.receiver_id.to_string(),
                                created_by_receipt_id: None,
                                deleted_by_receipt_id: Some(receipt.receipt_id.to_string()),
                                created_by_block_height: Default::default(),
                                deleted_by_block_height: Some(BigDecimal::from(block_height)),
                            });
                    }
                    _ => {}
                }
            }
        }
    }

    let (accounts_to_create, accounts_to_update): (
        Vec<models::accounts::Account>,
        Vec<models::accounts::Account>,
    ) = accounts
        .values()
        .cloned()
        .partition(|model| model.deleted_by_receipt_id.is_none());

    let create_accounts_future =
        async { models::chunked_insert(pool, &accounts_to_create, 10).await };

    let update_accounts_future =
        async { models::update_retry_or_panic(pool,
                                              "UPDATE accounts SET deleted_by_receipt_id = $3, deleted_by_block_height = $5\n\
            WHERE account_id = $1 AND deleted_by_receipt_id IS NULL",
                                              &accounts_to_update, 10).await };

    try_join!(create_accounts_future, update_accounts_future)?;
    Ok(())
}
