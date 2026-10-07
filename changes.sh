#!/bin/bash
set -euxo pipefail

git diff --word-diff $(cat ./last-reconfig.txt) gate-keys.txt
