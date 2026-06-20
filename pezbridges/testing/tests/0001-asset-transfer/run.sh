#!/usr/bin/env bash

set -e

source "${BASH_SOURCE%/*}/../../framework/utils/common.sh"
source "${BASH_SOURCE%/*}/../../framework/utils/zombienet.sh"

export ENV_PATH=`realpath ${BASH_SOURCE%/*}/../../environments/pezkuwichain-zagros`

$ENV_PATH/spawn.sh --init --start-relayer &
env_pid=$!

ensure_process_file $env_pid $TEST_DIR/pezkuwichain.env 600
pezkuwichain_dir=`cat $TEST_DIR/pezkuwichain.env`
echo

ensure_process_file $env_pid $TEST_DIR/zagros.env 300
zagros_dir=`cat $TEST_DIR/zagros.env`
echo

run_zndsl ${BASH_SOURCE%/*}/roc-relayer-balance-does-not-change.zndsl $pezkuwichain_dir
run_zndsl ${BASH_SOURCE%/*}/wnd-relayer-balance-does-not-change.zndsl $zagros_dir

run_zndsl ${BASH_SOURCE%/*}/roc-reaches-zagros.zndsl $zagros_dir
run_zndsl ${BASH_SOURCE%/*}/wnd-reaches-pezkuwichain.zndsl $pezkuwichain_dir

run_zndsl ${BASH_SOURCE%/*}/wroc-reaches-pezkuwichain.zndsl $pezkuwichain_dir
run_zndsl ${BASH_SOURCE%/*}/wwnd-reaches-zagros.zndsl $zagros_dir

run_zndsl ${BASH_SOURCE%/*}/roc-relayer-balance-does-not-change.zndsl $pezkuwichain_dir
run_zndsl ${BASH_SOURCE%/*}/wnd-relayer-balance-does-not-change.zndsl $zagros_dir
