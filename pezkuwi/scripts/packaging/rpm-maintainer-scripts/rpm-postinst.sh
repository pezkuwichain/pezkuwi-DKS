#!/bin/sh
# Post-install script for RPM package

set -e

config_file="/etc/default/pezkuwi"

# Create pezkuwi group if it doesn't exist
getent group pezkuwi >/dev/null || groupadd -r pezkuwi

# Create pezkuwi user if it doesn't exist
getent passwd pezkuwi >/dev/null || \
    useradd -r -g pezkuwi -d /home/pezkuwi -m -s /sbin/nologin \
    -c "User account for running pezkuwi as a service" pezkuwi

# Create default config file if it doesn't exist
if [ ! -e "$config_file" ]; then
    echo 'PEZKUWI_CLI_ARGS=""' > "$config_file"
fi

# Set correct permissions for binaries and service files
echo "Setting file permissions..."
chmod 755 /usr/bin/pezkuwi || true
chmod 755 /usr/lib/pezkuwi || true
chmod 755 /usr/lib/pezkuwi/* || true
chmod 644 /usr/lib/systemd/system/pezkuwi.service || true

# Reload systemd daemon to recognize the new service
if command -v systemctl >/dev/null 2>&1; then
    systemctl daemon-reload || true
fi

exit 0
