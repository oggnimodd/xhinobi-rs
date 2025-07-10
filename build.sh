#!/bin/bash
set -e

echo "Building xhinobi..."
cargo build --release

echo "Installing to ~/bin/xhinobi..."
mkdir -p ~/bin
cp target/release/xhinobi ~/bin/xhinobi

echo "Making executable..."
chmod +x ~/bin/xhinobi

echo "Build finished!"
echo "Make sure ~/bin is in your PATH"