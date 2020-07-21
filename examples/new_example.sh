#!/bin/bash

set -euo pipefail

NAME="$1"

cargo new "$NAME"

cd "$NAME"

echo 'psp = { path = "../../psp" }' >> Cargo.toml

cat > src/main.rs <<'EOF'
#![no_std]
#![no_main]

psp::module!("new_example", 1, 1);

fn psp_main() {
    psp::enable_home_button();
    psp::dprint!("Hello PSP!");
}
EOF

echo "Success! Created: \"$NAME\""
