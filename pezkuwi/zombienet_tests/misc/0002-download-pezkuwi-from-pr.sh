#!/usr/bin/env bash

set -euxo pipefail

echo $@

CFG_DIR=/cfg

# add CFG_DIR as first `looking dir` to allow to overrides commands.
mkdir -p $CFG_DIR
export PATH=$CFG_DIR:$PATH

cd $CFG_DIR
# see 0002-upgrade-node.zndsl to view the args.
curl -L -O $1/pezkuwi &
curl -L -O $1/pezkuwi-prepare-worker &
curl -L -O $1/pezkuwi-execute-worker &
wait

chmod +x $CFG_DIR/pezkuwi $CFG_DIR/pezkuwi-prepare-worker $CFG_DIR/pezkuwi-execute-worker
echo $(pezkuwi --version)
