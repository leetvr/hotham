#!/bin/bash
set -e
echo "All your OpenXR are belong to crab"
cargo build
cd ../openxrs/openxr
cargo run --example vulkan