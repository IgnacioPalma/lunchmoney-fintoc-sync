use std::io::BufRead;

use anyhow::anyhow;
use anyhow::bail;
use anyhow::Context;
use anyhow::Result;
use chrono::{DateTime, Utc};
use dialoguer::{Confirm, Input, Password};
use hyper::header::{AUTHORIZATION, CONTENT_TYPE, COOKIE};
use hyper::{body, body::Buf, Method, Request, StatusCode};
use rusty_money::iso::Currency;
use serde_json::{json, Value};

use crate::types::fintoc::Account;
use crate::types::fintoc::{AccountCredentials, Institution, Movement, TransferAccount};
use crate::types::lunchmoney::Amount;
use crate::types::HttpsClient;

pub async fn fetch_fintoc_movements(
    client: &HttpsClient,
    credentials: &AccountCredentials,
    start_date: DateTime<Utc>,
    end_date: DateTime<Utc>,
) -> Result<Vec<Movement>> {
    // Pagination
    let mut page = 1;
    let mut movements = Vec::new();

    // Paginate using the `Link` header
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

    Ok((
        Amount(account.balance.current as f64),
        *rusty_money::iso::find(&account.currency)
            .ok_or_else(|| anyhow!("Given currency {} is not valid", account.currency))?,
    ))
}
