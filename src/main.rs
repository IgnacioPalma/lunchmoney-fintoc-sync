use anyhow::Result;
use chrono::offset::{Local, Utc};
use chrono::DateTime;
use clap::{Parser, Subcommand};
use config::Config;
use hyper::client::Client;
use hyper_tls::HttpsConnector;
use serde::Deserialize;
use indicatif::{ProgressBar, ProgressStyle};
use colored::*;

mod fintoc;
mod lunchmoney;
mod types;

use fintoc::fetch_fintoc_movements;
use itertools::Itertools;
use lunchmoney::{get_all_assets, insert_transactions, update_asset_balance};
use types::fintoc::AccountCredentials;
use types::lunchmoney::Transaction;
use types::HttpsClient;

#[derive(Debug, Deserialize)]
struct AppConfig {
    tokens: Tokens,
    banks: Vec<Bank>,
    sync_settings: SyncSettings,
}

#[derive(Debug, Deserialize)]
struct Tokens {
    fintoc_secret_token: String,
    lunch_money_api_token: String,
}

#[derive(Debug, Deserialize)]
struct Bank {
    name: String,
    link_token: String,
    accounts: Vec<Account>,
}

#[derive(Debug, Deserialize)]
struct Account {
    name: String,
    fintoc_account_id: String,
    lunch_money_asset_id: String,
}

#[derive(Debug, Deserialize)]
struct SyncSettings {
    default_start_from: String,
}

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
struct Cmd {
    #[clap(subcommand)]
    verb: Verb,

    #[clap(long, default_value = "config.toml")]
    config: String,

    #[clap(long)]
    debug: bool,
}


#[derive(Subcommand)]
enum Verb {
    ListMovements {
        #[clap(default_value = "")]
        bank_name: String,
        #[clap(default_value = "")]
        account_name: String,
    },
    ListLunchMoneyAssets,
    SyncFintocMovements {
        #[clap(default_value = "")]
        bank_name: String,
        #[clap(default_value = "")]
        account_name: String,
    },
}

async fn cmd_list_fintoc_transactions(
    client: &HttpsClient,
    config: &AppConfig,
    bank_name: &str,
    account_name: &str,
    debug: bool,
) -> Result<()> {
    let banks_to_list = if bank_name.is_empty() {
        config.banks.iter().collect::<Vec<_>>()
    } else {
        config
            .banks
            .iter()
            .filter(|b| b.name == bank_name)
            .collect::<Vec<_>>()
    };

    let end_date: DateTime<Utc> = Local::now().into();
    let start_date: DateTime<Utc> = (Local::now()
        - chrono::Duration::from_std(
            humantime::parse_duration(&config.sync_settings.default_start_from).unwrap(),
        )
        .unwrap())
    .into();

    println!(
        "{}",
        format!(
            "Time period: {} UTC to {} UTC",
            start_date.format("%Y-%m-%d %H:%M:%S"),
            end_date.format("%Y-%m-%d %H:%M:%S"),
        )
        .bold()
    );

    for bank in banks_to_list {
        let accounts_to_list = if account_name.is_empty() {
            bank.accounts.iter().collect::<Vec<_>>()
        } else {
            bank.accounts
                .iter()
                .filter(|a| a.name == account_name)
                .collect::<Vec<_>>()
        };

        for account in accounts_to_list {
            println!("{}", format!("Listing movements for {} - {}", bank.name, account.name).bold());

            let credentials = AccountCredentials {
                account_id: account.fintoc_account_id.clone(),
                secret_token: config.tokens.fintoc_secret_token.clone(),
                link_token: bank.link_token.clone(),
            };

            let movements = fetch_fintoc_movements(client, &credentials, start_date, end_date).await?;

            for movement in movements {
                if debug {
                    println!("{}", format!("{:?}", movement).cyan());
                } else {
                    println!(
                        "{}",
                        format!(
                            "Date: {}, Description: {}, Amount: {} {}",
                            movement.post_date,
                            movement.description,
                            movement.amount,
                            movement.currency
                        )
                        .cyan()
                    );
                }
            }
        }
    }

    Ok(())
}


async fn cmd_list_lunch_money_assets(client: &HttpsClient, config: &AppConfig) -> Result<()> {
    let assets = get_all_assets(client, &config.tokens.lunch_money_api_token).await?;
    println!("{:#?}", assets);
    Ok(())
}

async fn cmd_sync_fintoc_movements(
    client: &HttpsClient,
    config: &AppConfig,
    bank_name: &str,
    account_name: &str,
) -> Result<()> {
    let end_date: DateTime<Utc> = Local::now().into();
    let start_date: DateTime<Utc> = (Local::now()
        - chrono::Duration::from_std(
            humantime::parse_duration(&config.sync_settings.default_start_from).unwrap(),
        )
        .unwrap())
    .into();

    let banks_to_sync = if bank_name.is_empty() {
        config.banks.iter().collect::<Vec<_>>()
    } else {
        config
            .banks
            .iter()
            .filter(|b| b.name == bank_name)
            .collect::<Vec<_>>()
    };

    for bank in banks_to_sync {
        let accounts_to_sync = if account_name.is_empty() {
            bank.accounts.iter().collect::<Vec<_>>()
        } else {
            bank.accounts
                .iter()
                .filter(|a| a.name == account_name)
                .collect::<Vec<_>>()
        };

        for account in accounts_to_sync {
            println!("{}", format!("Syncing {} - {}", bank.name, account.name).bold());

            let credentials = AccountCredentials {
                account_id: account.fintoc_account_id.clone(),
                secret_token: config.tokens.fintoc_secret_token.clone(),
                link_token: bank.link_token.clone(),
            };

            let (balance_amount, balance_currency) =
                fintoc::fetch_fintoc_balance(client, &credentials).await?;

            println!(
                "{}",
                format!("Found current account balance: {} {}", balance_amount, balance_currency)
                    .green()
            );

            let movements =
                fetch_fintoc_movements(client, &credentials, start_date, end_date).await?;

            println!("{}", format!("Fetched a total of {} movements.", movements.len()).blue());

            let pb = ProgressBar::new(movements.len() as u64);
            pb.set_style(
                ProgressStyle::default_bar()
                    .template("{msg}\n{wide_bar} {pos}/{len} ({eta})")?
                    .progress_chars("=>-")
            );

            let lunchmoney_transactions = movements
                .into_iter()
                .filter_map(|movement| {
                    account.lunch_money_asset_id.parse::<u64>().ok().and_then(|asset_id| {
                        movement.to_lunchmoney_transaction(asset_id).ok()
                    })
                })
                .collect::<Vec<Transaction>>();

            let mut synced_transactions: Vec<u64> = Vec::new();
            let mut existing_count = 0;

            for transaction_chunk in &lunchmoney_transactions.into_iter().chunks(50) {
                let (ids, existing_count_chunk) = insert_transactions(
                    client,
                    &config.tokens.lunch_money_api_token,
                    transaction_chunk.collect(),
                )
                .await?;

                existing_count += existing_count_chunk;
                synced_transactions.extend(ids);
                pb.set_message(format!("Processing chunk..."));
                pb.inc(50);
            }

            pb.finish_with_message("Sync complete!");

            println!(
                "{}",
                format!(
                    "Sync complete: {} new transactions were inserted, and {} already existed.",
                    synced_transactions.len(),
                    existing_count
                )
                .green()
            );

            update_asset_balance(
                client,
                &config.tokens.lunch_money_api_token,
                account.lunch_money_asset_id.parse()?,
                balance_amount,
                balance_currency,
            )
            .await?;

            println!("{}", "Updated asset balance successfully.".green());
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let cmd = Cmd::parse();

    let config = Config::builder()
        .add_source(config::File::with_name(&cmd.config))
        .build()?;

    let config: AppConfig = config.try_deserialize()?;

    let https = HttpsConnector::new();
    let client = Client::builder().build::<_, hyper::Body>(https);

    match cmd.verb {
        Verb::ListMovements {
            bank_name,
            account_name,
        } => cmd_list_fintoc_transactions(&client, &config, &bank_name, &account_name, cmd.debug).await,
        Verb::ListLunchMoneyAssets => cmd_list_lunch_money_assets(&client, &config).await,
        Verb::SyncFintocMovements {
            bank_name,
            account_name,
        } => cmd_sync_fintoc_movements(&client, &config, &bank_name, &account_name).await,
    }
}
