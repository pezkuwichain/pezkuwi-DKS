#!/usr/bin/env bash

set -e

source "${BASH_SOURCE%/*}/../../framework/utils/common.sh"
source "${BASH_SOURCE%/*}/../../framework/utils/zombienet.sh"

export ENV_PATH=`realpath ${BASH_SOURCE%/*}/../../environments/pezkuwichain-zagros`

$ENV_PATH/spawn.sh &
env_pid=$!

ensure_process_file $env_pid $TEST_DIR/pezkuwichain.env 600
pezkuwichain_dir=`cat $TEST_DIR/pezkuwichain.env`
echo

ensure_process_file $env_pid $TEST_DIR/zagros.env 300
zagros_dir=`cat $TEST_DIR/zagros.env`
echo

# Sleep for some time before starting the relayer. We want to sleep for at least 1 session,
# which is expected to be 60 seconds for the test environment.
echo -e "Sleeping 90s before starting relayer ...\n"
sleep 90
${BASH_SOURCE%/*}/../../environments/pezkuwichain-zagros/start_relayer.sh $pezkuwichain_dir $zagros_dir finality_relayer_pid teyrchains_relayer_pid messages_relayer_pid

run_zndsl ${BASH_SOURCE%/*}/pezkuwichain-to-zagros.zndsl $zagros_dir
run_zndsl ${BASH_SOURCE%/*}/zagros-to-pezkuwichain.zndsl $pezkuwichain_dir

