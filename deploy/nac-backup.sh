#!/bin/sh
set -eu

DEST=${NAC_ROOT:-/srv/nac}
OUT=${NAC_BACKUP_DIR:-/root/backups/nac}
umask 077

mkdir -p "$OUT"
stamp=$(date +%Y%m%d-%H%M%S)
tmp="$OUT/.nac-$stamp.tgz"
tar czf "$tmp" -C "$DEST" site admin/site data/appdata archive
mv "$tmp" "$OUT/nac-$stamp.tgz"
ls -1t "$OUT"/nac-*.tgz 2>/dev/null | tail -n +8 | xargs -r rm -f
