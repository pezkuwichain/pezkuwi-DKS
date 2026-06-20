#!/usr/bin/env bash

cd ../ethereum

truffle exec scripts/dumpTeyrchainConfig.js | sed '/^Using/d;/^$/d'
