# lunchmoney-fintoc-syncer (Based on Agucova's work on [GitHub](https://github.com/agucova/lunchmoney-fintoc-sync))

> Sadly, I got lots of issues when forking the original repo, so I decided to create a new one from scratch. This is a fork of [agucova/lunchmoney-fintoc-sync](https://github.com/agucova/lunchmoney-fintoc-sync)

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

### Fintoc Setup

1. Go to [Fintoc's dashboard](https://app.fintoc.com/) and create an account.
2. Connect your Chilean bank account through their interface to get a **link token**.
3. Get your **secret token** from the Fintoc settings/API keys section.

### Project Setup

1. Setup the Rust toolchain locally. I recommend using [rustup.rs](https://rustup.rs). You should now be able to run `cargo` in your terminal.
2. Clone this repo somewhere and `cd` to it, e.g. `git clone https://github.com/agucova/lunchmoney-fintoc-syncer.git && cd lunchmoney-fintoc-syncer`.
3. Create a `config.toml` file in the project root with your API tokens and bank configuration:

```toml
[tokens]
fintoc_secret_token = "YOUR_FINTOC_SECRET_TOKEN"
lunch_money_api_token = "YOUR_LUNCH_MONEY_API_TOKEN"

[[banks]]
name = "your_bank_name"
link_token = "YOUR_FINTOC_LINK_TOKEN"

[[banks.accounts]]
name = "your_account_name"
fintoc_account_id = "YOUR_FINTOC_ACCOUNT_ID"  # This will be obtained in step 5
lunch_money_asset_id = "YOUR_LUNCH_MONEY_ASSET_ID"  # This will be obtained in step 4
type = "Checking"  # or "Savings" or "Credit"

[sync_settings]
default_start_from = "30d"
```

4. **Get your Lunch Money asset ID**: Run `cargo run assets` to list your Lunch Money assets. Find the asset corresponding to the manually-managed asset you created earlier and note its ID.

5. **Get your Fintoc account ID**:
   - First, update your `config.toml` with your Fintoc secret token and link token from steps above
   - Run the helper script: `./get_accounts.sh` (it will be executable automatically)
   - This will show you the available accounts with their correct IDs (they start with `acc_`)
   - Copy the correct account ID from the output and update your `config.toml`

**Important**: The `fintoc_account_id` must start with `acc_` (not the link token). If you use the wrong ID, you'll get a "404 Not Found" error when syncing.

## Available Commands

The application has several helpful commands:

### List Lunch Money Assets

```bash
cargo run assets
```

Lists all your Lunch Money assets with their IDs and balances. Use this to find the asset ID you need for your config.

### Get Fintoc Account Information

```bash
./get_accounts.sh
```

Fetches and displays all accounts associated with your Fintoc link token. This shows the correct account IDs (starting with `acc_`) that you need for your configuration.

### List Bank Movements

```bash
cargo run movements [bank_name] [account_name]
```

Lists transactions from your bank account via Fintoc. Leave bank_name and account_name empty to list all configured accounts.

### Sync Transactions

```bash
cargo run sync [bank_name] [account_name]
```

Syncs transactions from your bank account to Lunch Money. Leave bank_name and account_name empty to sync all configured accounts.

## Configuration

All configuration is done via the `config.toml` file. Here's a complete example:

```toml
[tokens]
fintoc_secret_token = "sk_live_Bwb2ywks55z7jvsyhuSJAA72TzpZib7eNqSrcoEsAzx"
lunch_money_api_token = "dde134c6e65071557b09f0780940ed8e41c3a04d049bc09236"

[[banks]]
name = "BancoEstado"
link_token = "link_0xVGPvixKJorLRMw_token_xAs6j8cyczAyQHTxvkzmrFYg"

[[banks.accounts]]
name = "Cuenta Vista"
fintoc_account_id = "acc_8XkZle9Tx6RwlVQ6"  # Obtained from ./get_accounts.sh
lunch_money_asset_id = "168494"              # Obtained from cargo run assets
type = "Checking"

[sync_settings]
default_start_from = "30d"  # Can be "1d", "7d", "30d", etc.
```

### Configuration Options

- **Multiple banks and accounts**: You can add multiple `[[banks]]` sections and multiple `[[banks.accounts]]` under each bank
- **Time ranges**: Set `default_start_from` to control how far back to sync (e.g., "1d", "7d", "30d")
- **Skip movements**: Add `skip_movements = true` to an account to only sync balance without transactions
- **Account types**: Use "Checking", "Savings", or "Credit" to match your account type

The sync will fetch transactions from the configured time period and insert any new transactions into your Lunch Money asset, then update the asset balance to match your bank account balance.

## Troubleshooting

### Common Issues

**Error: "No such account: link_xxx. account objects start with acc prefix"**

- This means you're using the link token instead of the account ID in your config
- Run `./get_accounts.sh` to get the correct account ID (starts with `acc_`)
- Update the `fintoc_account_id` in your `config.toml`

**Error: "404 Not Found" when syncing**

- Double-check that your `fintoc_account_id` is correct (starts with `acc_`)
- Verify your `fintoc_secret_token` and `link_token` are valid
- Make sure your Fintoc link is still active

**No transactions found**

- Check if the date range is correct (default is last 30 days)
- Verify that your bank account actually has transactions in that period
- Try running `cargo run movements` to see if Fintoc can fetch transactions

### Getting Help

If you encounter issues:

1. Check that all your tokens and IDs are correct in `config.toml`
2. Use the `./get_accounts.sh` script to verify your account setup
3. Try the individual commands (`assets`, `movements`) before running `sync`

## Automation

This tool currently requires manual execution (`cargo run sync`), but you can automate it using system scheduling or GitHub Actions.

### GitHub Actions (Recommended)

The repository includes a GitHub Action that runs every 6 hours automatically. To set it up:

1. **Fork or push this repository to GitHub**
2. **Go to your repository Settings → Secrets and variables → Actions**
3. **Add the following repository secrets:**

   - `FINTOC_SECRET_TOKEN`: Your Fintoc secret token
   - `LUNCH_MONEY_API_TOKEN`: Your Lunch Money API token
   - `BANK_NAME`: Your bank name (e.g., "BancoEstado")
   - `FINTOC_LINK_TOKEN`: Your Fintoc link token
   - `ACCOUNT_NAME`: Your account name (e.g., "Cuenta Vista")
   - `FINTOC_ACCOUNT_ID`: Your Fintoc account ID (starts with `acc_`)
   - `LUNCH_MONEY_ASSET_ID`: Your Lunch Money asset ID
   - `ACCOUNT_TYPE`: Your account type ("Checking", "Savings", or "Credit")

4. **Enable GitHub Actions** in your repository settings if not already enabled
5. **The workflow will run automatically** every 6 hours at 00:00, 06:00, 12:00, and 18:00 UTC
6. **Manual trigger**: You can also run it manually from the Actions tab → "Lunch Money Fintoc Sync" → "Run workflow"
