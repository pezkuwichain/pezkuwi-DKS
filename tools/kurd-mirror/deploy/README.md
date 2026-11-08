# kurd-mirror on a gateway

```
install -m 755 kurd-mirror /usr/local/bin/
install -d /etc/ipfs/denylists
# kubo watches only files that exist when it starts; create the file, then restart it once
kurd-mirror deny --rpc "$RPC" --restart true && systemctl restart ipfs
echo "KURD_MIRROR_RPC=$RPC" > /etc/kurd-mirror.env
cp kurd-mirror-deny.service kurd-mirror-deny.timer /etc/systemd/system/
systemctl daemon-reload && systemctl enable --now kurd-mirror-deny.timer
```

Check: `journalctl -u kurd-mirror-deny -n 5` prints one line per run (block, names,
suspensions, refused roots, rules). A refused CID answers `410 Gone` at the gateway.
