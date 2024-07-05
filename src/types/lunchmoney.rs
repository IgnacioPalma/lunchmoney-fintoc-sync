use std::fmt;
use std::num::ParseFloatError;
use std::str::FromStr;
use std::time::UNIX_EPOCH;

use chrono::{DateTime, Utc};
use colored::*;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, skip_serializing_none, DisplayFromStr};

use currency_rs::{Currency, CurrencyOpts};

/// Tag object as described in https://lunchmoney.dev/#tags-object.
#[derive(Debug, Serialize)]
pub struct Tag {
    pub id: u64,
    pub name: String,
    pub description: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
#[allow(dead_code)]
pub enum TransactionStatus {
    Cleared,
    Uncleared,
}

/// An f64 that serializes to a float up to 4 decimal places, as specified in the `Transaction`
/// amount field description in https://lunchmoney.dev/#transaction-object.
/// TODO: Verify the sanity of using floats over decimals for currency amounts.
#[derive(Debug, PartialEq, Clone, Copy)]
pub struct Amount(pub f64);

impl FromStr for Amount {
    type Err = ParseFloatError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Amount(s.parse::<f64>()?))
    }
}

impl fmt::Display for Amount {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:.4}", self.0)
    }
}

impl From<f64> for Amount {
    fn from(val: f64) -> Self {
        Amount(val)
    }
}

/// Transaction object as defined in https://lunchmoney.dev/#transaction-object
#[serde_as]
#[skip_serializing_none]
#[derive(Debug, Serialize)]
pub struct Transaction {
    pub id: Option<u64>,
    pub date: DateTime<Utc>,
    // This is the display name of the payee
    pub payee: Option<String>,
    #[serde_as(as = "DisplayFromStr")]
    pub amount: Amount,
    // Lowercase ISO 4217 currency code
    pub currency: Option<String>,

    pub category_id: Option<u64>,
    pub asset_id: Option<u64>,
    pub status: TransactionStatus,
    pub parent_id: Option<u64>,
    pub is_group: Option<bool>,
    pub group_id: Option<u64>,
    pub tags: Option<Vec<Tag>>,
    pub external_id: Option<String>,
    pub notes: Option<String>,
    pub original_name: Option<String>,
    pub is_pending: Option<bool>,
}

impl Default for Transaction {
    fn default() -> Self {
        Self {
            id: None,
            date: UNIX_EPOCH.into(),
            payee: None,
            amount: Amount(0.0),
            currency: None,
            notes: None,
            category_id: None,
            asset_id: None,
            status: TransactionStatus::Uncleared,
            parent_id: None,
            is_group: None,
            group_id: None,
            tags: None,
            external_id: None,
            original_name: None,
            is_pending: None,
        }
    }
}

impl Transaction {
    pub fn to_colored_string(&self) -> ColoredString {
        let payee = match &self.payee {
            Some(payee) => payee.clone(),
            None => "Unknown".to_string(),
        };

        let opt = match &self.currency {
            Some(currency) => match currency.to_uppercase().as_str() {
                "USD" => CurrencyOpts::new()
                    .set_symbol("$")
                    .set_precision(2)
                    .set_from_cents(false),
                "EUR" => CurrencyOpts::new()
                    .set_symbol("€")
                    .set_precision(2)
                    .set_from_cents(false),
                "CLP" => CurrencyOpts::new()
                    .set_symbol("$")
                    .set_precision(0)
                    .set_from_cents(false),
                _ => CurrencyOpts::default(),
            },
            None => CurrencyOpts::default(),
        };

        let currency: Currency = Currency::new_float(self.amount.0, Some(opt));

        let amount = match self.amount.0 {
            amount if amount >= 0.0 => currency.format().green(),
            _ => currency.format().red(),
        };

        let currency_unit = &self
            .currency
            .as_ref()
            .unwrap_or(&"UNK".to_string())
            .to_uppercase();

        let datetime_str = self.date.format("%Y-%m-%d").to_string();

        format!(
            "{} - {}: {} ({})",
            datetime_str, payee, amount, currency_unit
        )
        .normal()
    }
}

#[serde_as]
#[derive(Debug, Deserialize, Serialize)]
pub struct Asset {
    pub id: Option<u64>,
    #[serde(rename = "type_name")]
    pub type_: Option<String>,
    #[serde(rename = "subtype_name")]
    pub subtype: Option<String>,
    pub name: Option<String>,
    pub display_name: Option<String>,
    #[serde_as(as = "DisplayFromStr")]
    pub balance: Amount,
    pub balance_as_of: Option<DateTime<Utc>>,
    pub closed_on: Option<String>,
    pub currency: String,
    pub institution_name: Option<String>,
    pub exclude_transactions: Option<bool>,
    pub created_at: Option<DateTime<Utc>>,
}

impl Default for Asset {
    fn default() -> Self {
        Self {
            id: None,
            type_: None,
            subtype: None,
            name: None,
            display_name: None,
            balance: Amount(0.0),
            balance_as_of: None,
            closed_on: None,
            currency: "usd".to_string(),
            institution_name: None,
            exclude_transactions: None,
            created_at: None,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct GetAllAssetsResponse {
    pub assets: Vec<Asset>,
}

#[skip_serializing_none]
#[derive(Debug, Serialize)]
pub struct InsertTransactionRequest<'a> {
    pub transactions: Vec<&'a Transaction>,
    pub apply_rules: Option<bool>,
    pub skip_duplicates: Option<bool>,
    pub check_for_recurring: Option<bool>,
    pub debit_as_negative: Option<bool>,
    pub skip_balance_update: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct InsertTransactionResponse {
    pub ids: Option<Vec<u64>>,
    pub error: Option<Vec<String>>,
}
