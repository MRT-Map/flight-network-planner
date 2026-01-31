#!/bin/bash
set -euxo pipefail

git diff $(cat ./last-reconfig.txt) gates2.txt
