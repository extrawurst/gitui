#!/bin/bash
set -e -u -o pipefail
cd $GITHUB_WORKSPACE
bash -c "$*"