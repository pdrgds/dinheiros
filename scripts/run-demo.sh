#!/usr/bin/env bash
# Launch dinheiros against a freshly-seeded throwaway SQLite DB. Used for
# generating README/marketing screenshots without touching the real DB at
# ~/Library/Application Support/dinheiros/data.db.
#
# Override the target file by exporting DINHEIROS_DEMO_DB before invocation.

set -euo pipefail

cd "$(dirname "$0")/.."

DB="${DINHEIROS_DEMO_DB:-/tmp/dinheiros-demo.db}"

cargo run --quiet -p dinheiros-core --bin dinheiros-seed -- "$DB"
DINHEIROS_DB_PATH="$DB" cargo run --bin dinheiros
