#!/bin/bash
set -euxo pipefail

git diff $(cat ./last-reconfig.txt) gate-keys.txt
