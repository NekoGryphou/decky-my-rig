#!/bin/sh

set -eu
cd /plugin
cargo fetch --locked
cargo xtask agents /backend/out
