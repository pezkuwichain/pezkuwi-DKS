#!/usr/bin/env bash

set -e

trap "trap - SIGTERM && kill -9 -$$" SIGINT SIGTERM EXIT

source "$FRAMEWORK_PATH/utils/zombienet.sh"

# whether to init the chains (open HRMP channels, set XCM version, create reserve assets, etc)
init=0
start_relayer=0
while [ $# -ne 0 ]
do
    arg="$1"
    case "$arg" in
        --init)
            init=1
            ;;
        --start-relayer)
            start_relayer=1
            ;;
    esac
    shift
done

logs_dir=$TEST_DIR/logs
helper_script="${BASH_SOURCE%/*}/helper.sh"

pezkuwichain_def=${BASH_SOURCE%/*}/bridge_hub_pezkuwichain_local_network.toml
start_zombienet $TEST_DIR $pezkuwichain_def pezkuwichain_dir pezkuwichain_pid
echo

zagros_def=${BASH_SOURCE%/*}/bridge_hub_zagros_local_network.toml
start_zombienet $TEST_DIR $zagros_def zagros_dir zagros_pid
echo

if [[ $init -eq 1 ]]; then
  run_zndsl ${BASH_SOURCE%/*}/pezkuwichain-start.zndsl $pezkuwichain_dir
  run_zndsl ${BASH_SOURCE%/*}/zagros-start.zndsl $zagros_dir

  pezkuwichain_init_log=$logs_dir/pezkuwichain-init.log
  echo -e "Setting up the pezkuwichain side of the bridge. Logs available at: $pezkuwichain_init_log\n"
  zagros_init_log=$logs_dir/zagros-init.log
  echo -e "Setting up the zagros side of the bridge. Logs available at: $zagros_init_log\n"

  $helper_script init-pezkuwichain-local >> $pezkuwichain_init_log 2>&1 &
  pezkuwichain_init_pid=$!
  $helper_script init-zagros-local >> $zagros_init_log 2>&1 &
  zagros_init_pid=$!
  wait $pezkuwichain_init_pid $zagros_init_pid

  run_zndsl ${BASH_SOURCE%/*}/pezkuwichain-init.zndsl $pezkuwichain_dir
  run_zndsl ${BASH_SOURCE%/*}/zagros-init.zndsl $zagros_dir

  $helper_script init-asset-hub-pezkuwichain-local >> $pezkuwichain_init_log 2>&1 &
  pezkuwichain_init_pid=$!
  $helper_script init-asset-hub-zagros-local >> $zagros_init_log 2>&1 &
  zagros_init_pid=$!
  wait $pezkuwichain_init_pid $zagros_init_pid

  $helper_script init-bridge-hub-pezkuwichain-local >> $pezkuwichain_init_log 2>&1 &
  pezkuwichain_init_pid=$!
  $helper_script init-bridge-hub-zagros-local >> $zagros_init_log 2>&1 &
  zagros_init_pid=$!
  wait $pezkuwichain_init_pid $zagros_init_pid
fi

if [[ $start_relayer -eq 1 ]]; then
  ${BASH_SOURCE%/*}/start_relayer.sh $pezkuwichain_dir $zagros_dir finality_relayer_pid teyrchains_relayer_pid messages_relayer_pid
fi

echo $pezkuwichain_dir > $TEST_DIR/pezkuwichain.env
echo $zagros_dir > $TEST_DIR/zagros.env
echo

wait -n $pezkuwichain_pid $zagros_pid $finality_relayer_pid $teyrchains_relayer_pid $messages_relayer_pid
kill -9 -$$
