use anyhow::anyhow;
use anyhow::bail;
use anyhow::Context;
use anyhow::Error;
use anyhow::Result;
use chrono::{DateTime, Utc};
use hyper::header::{AUTHORIZATION, CONTENT_TYPE};
use hyper::{body, Method, Request, StatusCode};
use rusty_money::iso::Currency;
use serde_json::Value;

use crate::types::fintoc::Account;
use crate::types::fintoc::{AccountCredentials, Movement};
use crate::types::lunchmoney::Amount;
use crate::types::HttpsClient;
use crate::AccountType;

pub async fn fetch_fintoc_movements(
    client: &HttpsClient,
    credentials: &AccountCredentials,
    start_date: DateTime<Utc>,
    end_date: DateTime<Utc>,
) -> Result<Vec<Movement>> {
    // Pagination
    let mut page = 1;
    let mut movements = Vec::new();

    loop {
        let request = Request::builder()
            .method(Method::GET)
            .uri(format!(
                "https://api.fintoc.com/v1/accounts/{}/movements?link_token={}&since={}&until={}&per_page=300&page={}",
                credentials.account_id,
                credentials.link_token,
                start_date.format("%Y-%m-%d"),
                end_date.format("%Y-%m-%d"),
                page
            ))
            .header(AUTHORIZATION, credentials.secret_token.clone())
            .header(CONTENT_TYPE, "application/json")
            .body(body::Body::empty())
            .context("Failed to build request")?;

        let response = client.request(request).await?;

        let status = response.status();
        let bytes = body::to_bytes(response).await?;

        if status != StatusCode::OK {
            bail!(
                "Failed to get Fintoc transactions, code {}, err:\n{:#?}",
                status,
                bytes
            );
        }

        let data: Value = serde_json::from_slice(&bytes)?;

        if !data.is_array() {
            bail!("Data is not an array");
        }

        let data = data.as_array().unwrap();

        // Deserialize the movements
        for movement in data {
            let movement: Movement = serde_json::from_value(movement.clone())?;
            movements.push(movement);
        }

        if data.is_empty() {
            break;
        } else {
            page += 1;
        }
    }

    Ok(movements)
}

pub async fn fetch_fintoc_balance(
    client: &HttpsClient,
    credentials: &AccountCredentials,
    account_type: AccountType,
) -> Result<(Amount, Currency)> {
    let request = Request::builder()
        .method(Method::GET)
        .uri(format!(
            "https://api.fintoc.com/v1/accounts/{}?link_token={}",
            credentials.account_id, credentials.link_token,
        ))
        .header(AUTHORIZATION, credentials.secret_token.clone())
        .header(CONTENT_TYPE, "application/json")
        .body(body::Body::empty())
        .context("Failed to build request")?;

    let response = client.request(request).await?;

    let status = response.status();
    let bytes = body::to_bytes(response).await?;

    if status != StatusCode::OK {
        bail!(
            "Failed to get Fintoc balance, code {}, err:\n{:#?}",
            status,
            bytes
        );
    }

    let account: Account = serde_json::from_slice(&bytes)?;

    let mut balance = 
    match account_type {
        AccountType::Checking => Amount(account.balance.current as f64),
        AccountType::Savings => Amount(account.balance.current as f64),
        AccountType::Credit => {
            Amount((account.balance.limit - account.balance.available) as f64)
        }
    };

    balance = match account.currency.to_uppercase().as_str() {
        "CLP" => balance,
        "USD" => Amount(balance.0 / 100.0),
        "EUR" => Amount(balance.0 / 100.0),
        _ => {
            bail!(
                "Currency {} is not supported.",
                account.currency.to_uppercase(),
            );

        }
    };

    Ok((
        balance,
        *rusty_money::iso::find(&account.currency)
            .ok_or_else(|| anyhow!("Given currency {} is not valid", account.currency))?,
    ))
}
