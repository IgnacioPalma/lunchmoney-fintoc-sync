use chrono::offset::{Local, Utc};
use chrono::DateTime;
use clap::{Args, Parser, Subcommand};
use hyper::client::Client;
use hyper_tls::HttpsConnector;
use std::time::Duration;

mod fintoc;
mod lunchmoney;
mod types;

use fintoc::fetch_fintoc_movements;
use types::fintoc::AccountCredentials;
use types::HttpsClient;
use lunchmoney::{get_all_assets, insert_transactions};

#[derive(Args)]
struct ListMovementsArgs {
    #[clap(long, value_parser = humantime::parse_duration, default_value = "30d")]
    start_from: Duration,

    #[clap(long, value_parser = humantime::parse_duration)]
    end_to: Option<Duration>,

    #[clap(long)]
    account_id: String,

    #[clap(long)]
    secret_token: String,

    #[clap(long)]
    link_token: String,
}

async fn cmd_list_fintoc_transactions(client: &HttpsClient, args: ListMovementsArgs) {
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
        account_id: args.account_id,
        secret_token: args.secret_token,
        link_token: args.link_token,
    };

    let movements = fetch_fintoc_movements(client, credentials, start_date, end_date).await.unwrap();

    for movement in movements {
        println!("{:?}", movement);
    }
}

async fn cmd_list_lunch_money_assets(client: &HttpsClient, api_token: String) {
    let assets = get_all_assets(client, &api_token).await;

    println!("{:#?}", assets);
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
        #[clap(long)]
        api_token: String,
    },
}

#[tokio::main]
async fn main() {
    let cmd = Cmd::parse();

    let https = HttpsConnector::new();
    let client = Client::builder().build::<_, hyper::Body>(https);

    match cmd.verb {
        Verb::ListMovements(args) => cmd_list_fintoc_transactions(&client, args).await,
        Verb::ListLunchMoneyAssets { api_token } => {
            cmd_list_lunch_money_assets(&client, api_token).await
        }
    }
}
