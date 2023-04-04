#!/bin/bash

set -e

cargo clippy -- -D warnings

cargo test --workspace --lib -- --nocapture

cargo test --test web_authentication -- --nocapture
