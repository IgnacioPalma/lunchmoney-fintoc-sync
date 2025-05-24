#!/bin/bash

# Script to get Fintoc account IDs from a link token
# Usage: ./get_accounts.sh

# Read tokens from config.toml
FINTOC_SECRET_TOKEN=$(grep 'fintoc_secret_token' config.toml | cut -d'"' -f2)
LINK_TOKEN=$(grep 'link_token' config.toml | head -1 | cut -d'"' -f2)

echo "Using link token: $LINK_TOKEN"
echo "Fetching accounts..."
echo

# Fetch accounts using Fintoc API v1 (since that's what your code uses)
curl -s -X GET \
  "https://api.fintoc.com/v1/accounts?link_token=$LINK_TOKEN" \
  -H "Authorization: $FINTOC_SECRET_TOKEN" \
  -H "Content-Type: application/json" | jq '.'