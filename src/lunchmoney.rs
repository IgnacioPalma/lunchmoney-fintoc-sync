use anyhow::bail;
use anyhow::Result;
use hyper::header::{AUTHORIZATION, CONTENT_TYPE};
use hyper::{body, Method, Request, StatusCode};
use rusty_money::iso::Currency;

use crate::types::lunchmoney::Amount;
use crate::types::lunchmoney::{
    Asset, GetAllAssetsResponse, InsertTransactionRequest, InsertTransactionResponse, Transaction,
};
use crate::types::HttpsClient;

pub async fn get_all_assets(client: &HttpsClient, api_token: &str) -> Result<Vec<Asset>> {
    let request = Request::builder()
        .method(Method::GET)
        .uri("https://dev.lunchmoney.app/v1/assets")
        .header(AUTHORIZATION, format!("Bearer {}", api_token))
        .body(body::Body::empty())
        .unwrap();

    let response = client.request(request).await?;

    let status = response.status();
    let bytes = body::to_bytes(response).await?;

    if status != StatusCode::OK {
        bail!(
            "Failed to get Lunch Money assets, code {}, err:\n{:#?}",
            status,
            bytes
        );
    }

    let response: GetAllAssetsResponse = serde_json::from_slice(&bytes)?;

    Ok(response.assets)
}

async fn insert_single_transaction(
    client: &HttpsClient,
    api_token: &str,
    transaction: &Transaction,
) -> Result<Option<u64>> {
    let request_body = InsertTransactionRequest {
        transactions: vec![transaction],
        apply_rules: Some(true),
        check_for_recurring: Some(true),
        debit_as_negative: Some(true),
        skip_balance_update: None,
        skip_duplicates: None,
    };

    let request = Request::builder()
        .method(Method::POST)
        .uri("https://dev.lunchmoney.app/v1/transactions")
        .header(AUTHORIZATION, format!("Bearer {}", api_token))
        .header(CONTENT_TYPE, "application/json; charset=utf-8")
        .body(serde_json::to_vec(&request_body)?.into())
        .unwrap();

    let response = client.request(request).await?;

    let status = response.status();
    let bytes = body::to_bytes(response).await?;

    if status != StatusCode::OK {
        bail!(
            "Failed to insert Lunch Money transaction, code {}, err:\n{:#?}",
            status,
            bytes
        );
    }

    let response: InsertTransactionResponse = serde_json::from_slice(&bytes)?;

    match response {
        InsertTransactionResponse {
            ids: Some(ids),
            error: None,
        } => Ok(ids.into_iter().next()),
        InsertTransactionResponse {
            ids: Some(ids),
            error: Some(errors),
        } => {
            for error in errors {
                if error.contains("already exists") {
                    return Ok(None); // Indicate that the transaction already exists
                } else {
                    eprintln!("Error: {}", error);
                }
            }
            Ok(ids.into_iter().next())
        }
        InsertTransactionResponse {
            ids: None,
            error: Some(errors),
        } => {
            for error in errors {
                if error.contains("already exists") {
                    return Ok(None); // Indicate that the transaction already exists
                } else {
                    eprintln!("Error: {}", error);
                }
            }
            Ok(None)
        }
        InsertTransactionResponse {
            ids: None,
            error: None,
        } => Ok(None),
    }
}

pub async fn insert_transactions(
    client: &HttpsClient,
    api_token: &str,
    transactions: Vec<Transaction>,
) -> Result<(Vec<u64>, u64)> {
    let mut inserted_ids = vec![];
    let mut existing_count = 0;

    for transaction in &transactions {
        match insert_single_transaction(client, api_token, transaction).await {
            Ok(Some(id)) => inserted_ids.push(id),
            Ok(None) => existing_count += 1, // Count existing transactions
            Err(err) => eprintln!("Failed to insert transaction: {:?}", err),
        }
    }

    Ok((inserted_ids, existing_count))
}
pub async fn update_asset_balance(
    client: &HttpsClient,
    api_token: &str,
    asset_id: u64,
    new_balance: Amount,
    balance_currency: Currency,
) -> Result<()> {
    let updated_asset = Asset {
        id: Some(asset_id),
        balance: new_balance,
        currency: balance_currency.to_string().to_lowercase(),
        ..Default::default()
    };

    let request = Request::builder()
        .method(Method::PUT)
        .uri(format!("https://dev.lunchmoney.app/v1/assets/{}", asset_id))
        .header(AUTHORIZATION, format!("Bearer {}", api_token))
        .header(CONTENT_TYPE, "application/json; charset=utf-8")
        .body(serde_json::to_vec(&updated_asset)?.into())
        .unwrap();

    let response = client.request(request).await?;

    let status = response.status();

    if status != StatusCode::OK {
        bail!(
            "Failed to update Lunch Money asset balance, code {}",
            status
        );
    }

    let bytes = body::to_bytes(response).await?;

    let updated_asset: Asset = serde_json::from_slice(&bytes)?;

    // Assert new balance = updated_asset.balance
    if updated_asset.balance != new_balance {
        bail!(
            "Failed to update Lunch Money asset balance, expected {:?}, got {:?}",
            new_balance,
            updated_asset.balance,
        );
    }
    // Assert new currency = updated_asset.currency
    if updated_asset.currency != balance_currency.to_string().to_lowercase() {
        bail!(
            "Failed to update Lunch Money asset balance, expected {}, got {}",
            balance_currency,
            updated_asset.currency,
        );
    }

    Ok(())
}
