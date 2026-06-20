#!/usr/bin/env bash

set -e

source "$FRAMEWORK_PATH/utils/common.sh"
source "$FRAMEWORK_PATH/utils/zombienet.sh"

pezkuwichain_dir=$1
zagros_dir=$2
__finality_relayer_pid=$3
__teyrchains_relayer_pid=$4
__messages_relayer_pid=$5

logs_dir=$TEST_DIR/logs
helper_script="${BASH_SOURCE%/*}/helper.sh"

# start finality relayer
finality_relayer_log=$logs_dir/relayer_finality.log
echo -e "Starting pezkuwichain-zagros finality relayer. Logs available at: $finality_relayer_log\n"
start_background_process "$helper_script run-finality-relay" $finality_relayer_log finality_relayer_pid

# start teyrchains relayer
teyrchains_relayer_log=$logs_dir/relayer_teyrchains.log
echo -e "Starting pezkuwichain-zagros teyrchains relayer. Logs available at: $teyrchains_relayer_log\n"
start_background_process "$helper_script run-teyrchains-relay" $teyrchains_relayer_log teyrchains_relayer_pid

# start messages relayer
messages_relayer_log=$logs_dir/relayer_messages.log
echo -e "Starting pezkuwichain-zagros messages relayer. Logs available at: $messages_relayer_log\n"
start_background_process "$helper_script run-messages-relay" $messages_relayer_log messages_relayer_pid

run_zndsl ${BASH_SOURCE%/*}/pezkuwichain-bridge.zndsl $pezkuwichain_dir
run_zndsl ${BASH_SOURCE%/*}/zagros-bridge.zndsl $zagros_dir

eval $__finality_relayer_pid="'$finality_relayer_pid'"
eval $__teyrchains_relayer_pid="'$teyrchains_relayer_pid'"
eval $__messages_relayer_pid="'$messages_relayer_pid'"
