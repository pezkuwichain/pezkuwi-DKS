#!/usr/bin/env bash

set -e

# TODO: This test doesn't work. It was added at a time when we couldn't run it because we didn't have the scafolding.
# It needs to be fixed. For the moment we keep it in the repo as it is since the idea has value.
# But we don't run it in the CI.

source "${BASH_SOURCE%/*}/../../framework/utils/common.sh"
source "${BASH_SOURCE%/*}/../../framework/utils/zombienet.sh"

export ENV_PATH=`realpath ${BASH_SOURCE%/*}/../../environments/pezkuwichain-zagros`

logs_dir=$TEST_DIR/logs

$ENV_PATH/spawn.sh --init &
env_pid=$!

ensure_process_file $env_pid $TEST_DIR/pezkuwichain.env 600
pezkuwichain_dir=`cat $TEST_DIR/pezkuwichain.env`
echo

ensure_process_file $env_pid $TEST_DIR/zagros.env 300
zagros_dir=`cat $TEST_DIR/zagros.env`
echo

echo "Sending message from Pezkuwichain to Zagros"
$ENV_PATH/helper.sh auto-log reserve-transfer-assets-from-asset-hub-pezkuwichain-local 5000000000000
echo

echo "Sending message from Zagros to Pezkuwichain"
$ENV_PATH/helper.sh auto-log reserve-transfer-assets-from-asset-hub-zagros-local 5000000000000
echo


# Start the relayer with a 30s delay
# We want to be sure that the messages won't be relayed before starting the js script in `pezkuwichain-to-zagros.zndsl`
start_relayer_log=$logs_dir/start_relayer.log
echo -e "The pezkuwichain-zagros relayer will be started in 30s. Logs will be available at: $start_relayer_log\n"
(sleep 30 && $ENV_PATH/start_relayer.sh \
  $pezkuwichain_dir $zagros_dir finality_relayer_pid teyrchains_relayer_pid messages_relayer_pid > $start_relayer_log)&

run_zndsl ${BASH_SOURCE%/*}/pezkuwichain-to-zagros.zndsl $zagros_dir

