#!/bin/bash
set -euxo pipefail

./flight-network-planner run config.yml -sro out.txt
./flight-network-planner gate-keys out.txt > gate-keys.txt
