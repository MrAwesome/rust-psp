#!/bin/bash

set -euo

export BUILD_ROOT="$(pwd)"

export CARGO_HOME="${BUILD_ROOT}"/.cargo
export XARGO_HOME="${BUILD_ROOT}"/.xargo

# XXX
echo FUCK
pwd
find
find | grep 'cargo-psp$'
# XXX

pushd repo/cargo-psp/
cargo build --release
popd

pushd repo/ci/tests
${BUILD_ROOT}/repo/cargo-psp/target/release/cargo-psp
popd

cp -r repo/ci/tests/target/mipsel-sony-psp/release/* release/
