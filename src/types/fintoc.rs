use chrono::{DateTime, Utc};
use serde::Deserialize;

use super::lunchmoney;

#[derive(Debug, Deserialize)]
pub struct Institution {
    pub id: String,
    pub name: String,
    pub country: String,
}

#[derive(Debug, Deserialize)]
pub struct Balance {
    pub available: i128,
    pub current: i128,
    pub limit: i128,
}

#[derive(Debug, Deserialize)]
pub struct Account {
    pub id: String,
    pub object: String,
    pub name: String,
    pub official_name: String,
    pub number: Option<String>,
    pub holder_id: String,
    pub holder_name: String,
    #[serde(rename = "type")]
    pub account_type: String,
    pub currency: String,
    pub balance: Balance,
    pub refreshed_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct TransferAccount {
    pub holder_id: String,
    pub holder_name: String,
    pub number: Option<String>,
    pub institution: Option<Institution>,
}

#[derive(Debug, Deserialize)]
pub struct Movement {
    pub id: String,
    pub object: String,
    pub amount: i32,
    pub post_date: DateTime<Utc>,
    pub description: String,
    pub transaction_date: Option<DateTime<Utc>>,
    pub currency: String,
    pub reference_id: Option<String>,
    #[serde(rename = "type")]
    pub movement_type: String,
    pub pending: bool,
    pub recipient_account: Option<TransferAccount>,
    pub sender_account: Option<TransferAccount>,
    pub comment: Option<String>,
}

// Strings for now
type Error = String;

impl Movement {
    pub fn to_lunchmoney_transaction(
        &self,
        asset_id: u64,
    ) -> Result<lunchmoney::Transaction, Error> {
        let amount = lunchmoney::Amount(self.amount as f64);

        let payee = match &self.recipient_account {
            // If holder_name & institution name are present, use both
            Some(account) => {
                if let Some(institution) = &account.institution {
                    format!("{} ({})", account.holder_name, institution.name)
                } else {
                    account.holder_name.clone()
                }
            }
            // Otherwise extract from description
            None => {
                // Strip common prefixes if present
                // TODO: Expand this list
                let re = regex::Regex::new(r#"^(?i)(COMPRA INTERNACIONAL|COMPRA NACIONAL|PAGO RECURRENTE|COMPRA INTER.)\s"#).unwrap();
                re.replace(&self.description, "").to_string()
            }
        };

        Ok(lunchmoney::Transaction {
            date: self.transaction_date.unwrap_or(self.post_date),
            payee: Some(payee),
            amount,
            currency: Some(self.currency.to_lowercase()),
            asset_id: Some(asset_id),
            notes: self.comment.clone(),
            external_id: Some(self.id.clone()),
            status: lunchmoney::TransactionStatus::Uncleared,
            original_name: Some(self.description.clone()),
            is_pending: Some(self.pending),
            ..Default::default()
        })
    }
}

pub struct AccountCredentials {
    pub secret_token: String,
    pub link_token: String,
    pub account_id: String,
}
