#!/bin/bash -ex

cargo fmt

RUST_BACKTRACE=full RUST_LOG=info cargo run
