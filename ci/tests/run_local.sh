#!/bin/bash

set -euo pipefail

RUST_PSP_LOC="$HOME/rust-psp"
TEST_LOC="$RUST_PSP_LOC/ci/tests"

cd "$TEST_LOC"

export PATH="$RUST_PSP_LOC/target/debug:$PATH" 
cargo psp

rm -f target/mipsel-sony-psp/debug/psp_output_file.log

PPSSPPHeadless -l --timeout=5 target/mipsel-sony-psp/debug/EBOOT.PBP
cat target/mipsel-sony-psp/debug/psp_output_file.log
