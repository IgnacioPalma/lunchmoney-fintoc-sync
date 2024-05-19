# lunchmoney-fintoc-syncer
A Rust CLI to sync transactions from Chilean banks with the [Lunch Money budget app](https://lunchmoney.app). This uses [Fintoc](https://fintoc.com/), which provides a free API for querying your bank transactions. This should work with pretty much any Chilean bank.

This is pretty much WIP; don't expect it to be particularly stable (or not mess up all your transactions).
 
## Description
This CLI basically:
1. Fetches a list of transactions from your Chilean bank account using Fintoc's [List movements endpoint](https://docs.fintoc.com/reference/movements-list) and the current balance in the account using the [Retrieve account](https://docs.fintoc.com/reference/accounts-retrieve) endpoint.
2. Then, it individually goes and inserts any transactions that don't currently exist in your Lunch Money asset, and then updates the asset's balance to match that of your account.

This CLI only allows you to sync the transactions on demand. You can define a range of time for selecting transactions.

## Setup
### Lunch Money
1. Create a "manually-managed asset" in Lunch Money to sync your transactions to. You can do this on the [accounts page](https://my.lunchmoney.app/accounts) -> "Add Account" -> Scroll down to the "manually-managed assets" section and select "Cash" -> Select 'Checking' (cuenta corriente) or 'Savings' (cuenta vista) -> configure the name as you desire.
2. Generate a Lunch Money API key. Go to the [developer page](https://my.lunchmoney.app/developers) and select "Request New Access Token". Copy this token to somewhere secure for later user.

### Project Setup
1. Setup the Rust toolchain locally. I recommend using [rustup.rs](https://rustup.rs). You should now be able to run `cargo` in your terminal.
2. Clone this repo somewhere and `cd` to it, e.g. `git clone https://github.com/agucova/lunchmoney-fintoc-syncer.git && cd lunchmoney-fintoc-syncer`.
3. Run `cargo run list-lunch-money-assets --api-token <LUNCHMONEY_API_TOKEN>` where `<LUNCHMONEY_API_TOKEN>` is the Lunch Money API token you generated earlier. Find the asset corresponding to the "manually-managed asset" you created earlier and make note of the ID of that asset.
4. Connect your bank through the Fintoc panel, and store its link token, plus your personal secret key (you can find this in settings). 

## Running the Command
The core command is `sync-fintoc-movements`:
```
❯ cargo run sync-fintoc-movements --help
USAGE:
    lunchmoney-fintoc sync-fintoc-movements [OPTIONS] --fintoc-secret-token <FINTOC_SECRET_TOKEN> --lunch-money-api-token <LUNCH_MONEY_API_TOKEN> <FINTOC_LINK_TOKEN> <FINTOC_ACCOUNT_ID> <LUNCH_MONEY_ASSET_ID>

ARGS:
    <FINTOC_LINK_TOKEN>       
    <FINTOC_ACCOUNT_ID>       
    <LUNCH_MONEY_ASSET_ID>    

OPTIONS:
        --end-to <END_TO>                                  
        --fintoc-secret-token <FINTOC_SECRET_TOKEN>        [env: FINTOC_SECRET_TOKEN=]
    -h, --help                                             Print help information
        --lunch-money-api-token <LUNCH_MONEY_API_TOKEN>    [env: LUNCH_MONEY_API_TOKEN=]
        --start-from <START_FROM>                          [default: 30d]
```

Here, you pass your Fintoc link token, your Fintoc account ID (for now, you need to use the API to retrieve this), the Lunch Money API token, and the Lunch Money asset ID. You can configure the date range you want to sync by also setting `--start-from DURATION` and/or `--end-to DURATION` (where date is a human-readable duration, like 2m or 10d).
