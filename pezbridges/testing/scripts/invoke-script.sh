#!/usr/bin/env bash

INVOKE_LOG=`mktemp -p $TEST_FOLDER invoke.XXXXX`

pushd $PEZKUWI_SDK_PATH/bridges/testing/environments/pezkuwichain-zagros
./bridges_pezkuwichain_zagros.sh $1 >$INVOKE_LOG 2>&1
popd
