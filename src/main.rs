use chrono::offset::{Local, Utc};
use chrono::DateTime;
use clap::{Args, Parser, Subcommand};
use hyper::client::Client;
use hyper_tls::HttpsConnector;
use std::time::Duration;

use anyhow::Result;

mod fintoc;
mod lunchmoney;
mod types;

use fintoc::fetch_fintoc_movements;
use itertools::Itertools;
use lunchmoney::{get_all_assets, insert_transactions};
use types::fintoc::AccountCredentials;
use types::lunchmoney::Transaction;
use types::HttpsClient;

use crate::lunchmoney::update_asset_balance;

#[derive(Args)]
struct ListMovementsArgs {
    #[clap(long, value_parser = humantime::parse_duration, default_value = "30d")]
    start_from: Duration,

    #[clap(long, value_parser = humantime::parse_duration)]
    end_to: Option<Duration>,

    #[clap(long, env)]
    fintoc_secret_token: String,

    #[clap(long)]
    fintoc_account_id: String,

    #[clap(long)]
    fintoc_link_token: String,
}

async fn cmd_list_fintoc_transactions(client: &HttpsClient, args: ListMovementsArgs) -> Result<()> {
    let end_date: DateTime<Utc> = {
        let mut end_date = Local::now();

        if let Some(duration) = args.end_to {
            end_date = end_date - chrono::Duration::from_std(duration).unwrap();
        }

        end_date.into()
    };

    let start_date: DateTime<Utc> =
        (Local::now() - chrono::Duration::from_std(args.start_from).unwrap()).into();

    let credentials = AccountCredentials {
        account_id: args.fintoc_account_id,
        secret_token: args.fintoc_secret_token,
        link_token: args.fintoc_link_token,
    };

    let movements = fetch_fintoc_movements(client, &credentials, start_date, end_date)
        .await
        .unwrap();

    for movement in movements {
        println!("{:?}", movement);
    }

    Ok(())
}

async fn cmd_list_lunch_money_assets(client: &HttpsClient, api_token: String) -> Result<()> {
    let assets = get_all_assets(client, &api_token).await;

    println!("{:#?}", assets);

    Ok(())
}

#[derive(Args)]
struct SyncFintocMovementsArgs {
    #[clap(long, value_parser = humantime::parse_duration, default_value = "30d")]
    start_from: Duration,

    #[clap(long, value_parser = humantime::parse_duration)]
    end_to: Option<Duration>,

    #[clap(long, env)]
    fintoc_secret_token: String,

    #[clap()]
    fintoc_link_token: String,

    #[clap()]
    fintoc_account_id: String,

    #[clap(long, env)]
    lunch_money_api_token: String,

    #[clap()]
    lunch_money_asset_id: u64,
}

async fn cmd_sync_fintoc_movements(
    client: &HttpsClient,
    args: SyncFintocMovementsArgs,
) -> Result<()> {
    let end_date: DateTime<Utc> = {
        let mut end_date = Local::now();

        if let Some(duration) = args.end_to {
            end_date = end_date - chrono::Duration::from_std(duration).unwrap();
        }

        end_date.into()
    };

    let start_date: DateTime<Utc> =
        (Local::now() - chrono::Duration::from_std(args.start_from).unwrap()).into();

    let credentials = AccountCredentials {
        account_id: args.fintoc_account_id,
        secret_token: args.fintoc_secret_token,
        link_token: args.fintoc_link_token,
    };

    let (balance_amount, balance_currency) =
        fintoc::fetch_fintoc_balance(client, &credentials).await?;

    println!(
        "Found current account balance: {} {}",
        balance_amount, balance_currency
    );

    let movements = fetch_fintoc_movements(client, &credentials, start_date, end_date)
        .await
        .unwrap();

    println!("Fetched a total of {} movements.", movements.len());
    for movement in &movements {
        println!("{:?}", movement);
    }

    let lunchmoney_transactions = movements
        .into_iter()
        .map(|movement| movement.to_lunchmoney_transaction(args.lunch_money_asset_id))
        .filter_map(Result::ok)
        .collect::<Vec<Transaction>>();

    let mut synced_transactions: Vec<u64> = Vec::new();
    let mut existing_count = 0;

    for transaction_chunk in &lunchmoney_transactions.into_iter().chunks(50) {
        let (ids, existing_count_chunk) = insert_transactions(
            client,
            &args.lunch_money_api_token,
            transaction_chunk.collect(),
        )
        .await?;

        existing_count += existing_count_chunk;

        synced_transactions.extend(ids);
    }
    println!(
        "Sync complete: {} new transactions were inserted, and {} already existed.",
        synced_transactions.len(),
        existing_count
    );

    // Update the asset balance
    update_asset_balance(
        client,
        &args.lunch_money_api_token,
        args.lunch_money_asset_id,
        balance_amount,
        balance_currency,
    )
    .await?;

    println!("Updated asset balance successfully.");

    Ok(())
}

// A CLI to sync movements from Fintoc to Lunch Money
#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
struct Cmd {
    #[clap(subcommand)]
    verb: Verb,
}

#[derive(Subcommand)]
enum Verb {
    ListMovements(ListMovementsArgs),
    ListLunchMoneyAssets {
        #[clap(long, env)]
        lunch_money_api_token: String,
    },
    SyncFintocMovements(SyncFintocMovementsArgs),
}

#[tokio::main]
async fn main() -> Result<()> {
    let cmd = Cmd::parse();

    let https = HttpsConnector::new();
    let client = Client::builder().build::<_, hyper::Body>(https);

    match cmd.verb {
        Verb::ListMovements(args) => cmd_list_fintoc_transactions(&client, args).await,
        Verb::ListLunchMoneyAssets {
            lunch_money_api_token,
        } => cmd_list_lunch_money_assets(&client, lunch_money_api_token).await,
        Verb::SyncFintocMovements(args) => cmd_sync_fintoc_movements(&client, args).await,
    }
}
