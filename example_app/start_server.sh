#!/bin/bash
set -eu
script_path=$( cd "$(dirname "${BASH_SOURCE[0]}")" ; pwd -P )
cd "$script_path/.."

# Starts a local web-server that serves the contents of the `doc/` folder,
# which is the folder to where the web version is compiled.

cargo install basic-http-server

echo "open http://localhost:8081"

(cd docs && basic-http-server --addr 127.0.0.1:8081 .)
# (cd docs && python3 -m http.server 8081 --bind 127.0.0.1)
