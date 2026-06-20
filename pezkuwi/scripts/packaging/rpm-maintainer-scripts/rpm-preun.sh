#!/bin/sh
# Pre-uninstall script for RPM package

set -e

# Stop and disable the service before uninstall (but not on upgrade)
if [ "$1" = "0" ]; then
    # $1 = 0 means uninstall (not upgrade)
    if command -v systemctl >/dev/null 2>&1; then
        systemctl --no-reload disable pezkuwi.service || true
        systemctl stop pezkuwi.service || true
    fi
fi

exit 0
