#!/bin/sh
# nac 一键部署：在仓库根目录运行，目标是 Linux 服务器（systemd）。
#
#   sudo ./deploy/install.sh                 # 装到 /srv/nac，公开站点只监听回环
#   sudo ./deploy/install.sh --bind-public   # 同上，但公开站点绑 0.0.0.0（会暴露服务器 IP）
#   sudo ./deploy/install.sh --dest /opt/nac
#
# 前提：已经 cargo build --release，或者用 --from 指定二进制所在目录。

set -eu

DEST=/srv/nac
PUBLIC_USER=nac-public
CONSOLE_USER=nac-console
SHARE_GROUP=nac-share
BIND=127.0.0.1
FROM=target/release
KEEP_SITE=0
UNITS=""

while [ $# -gt 0 ]; do
    case "$1" in
        --dest) DEST=$2; shift 2 ;;
        --public-user) PUBLIC_USER=$2; shift 2 ;;
        --console-user) CONSOLE_USER=$2; shift 2 ;;
        --group) SHARE_GROUP=$2; shift 2 ;;
        --from) FROM=$2; shift 2 ;;
        --keep-site) KEEP_SITE=1; shift ;;
        --bind-public) BIND=0.0.0.0; shift ;;
        -h|--help) sed -n '2,12p' "$0"; exit 0 ;;
        *) echo "未知参数：$1" >&2; exit 2 ;;
    esac
done

[ "$(id -u)" = 0 ] || { echo "需要 root（用 sudo 跑）" >&2; exit 1; }
[ "$PUBLIC_USER" != "$CONSOLE_USER" ] || {
    echo "公开站与运营台必须是两个不同的用户" >&2
    exit 1
}
for bin in contentd publish probe; do
    [ -x "$FROM/$bin" ] || { echo "找不到 $FROM/$bin，先 cargo build --release" >&2; exit 1; }
done
[ -f site/1.py ] || { echo "请在仓库根目录运行" >&2; exit 1; }
command -v python3 >/dev/null || { echo "服务器上需要 python3" >&2; exit 1; }
command -v openssl >/dev/null || { echo "服务器上需要 openssl" >&2; exit 1; }

echo "== 用户、组与目录"
getent group "$SHARE_GROUP" >/dev/null 2>&1 || groupadd -r "$SHARE_GROUP"
for spec in "$PUBLIC_USER:$PUBLIC_USER" "$CONSOLE_USER:$CONSOLE_USER"; do
    user=${spec%%:*}
    group=${spec##*:}
    getent group "$group" >/dev/null 2>&1 || groupadd -r "$group"
    if ! id -u "$user" >/dev/null 2>&1; then
        useradd -r -s /usr/sbin/nologin -d "$DEST" -M -g "$group" -G "$SHARE_GROUP" "$user"
    fi
    id -nG "$user" | tr ' ' '\n' | grep -qx "$SHARE_GROUP" || usermod -aG "$SHARE_GROUP" "$user"
done

install -d -m 0750 -o root -g "$SHARE_GROUP" "$DEST"
install -d -m 0755 -o root -g root "$DEST/bin"
install -d -m 0700 -o "$CONSOLE_USER" -g "$CONSOLE_USER" "$DEST/admin"
install -d -m 2770 -o "$PUBLIC_USER" -g "$SHARE_GROUP" \
    "$DEST/data" "$DEST/data/state" "$DEST/data/appdata" "$DEST/archive"
[ -e "$DEST/data/access.jsonl" ] || \
    install -m 0660 -o "$PUBLIC_USER" -g "$SHARE_GROUP" /dev/null "$DEST/data/access.jsonl"

echo "== TLS 证书"
install -d -m 0755 -o root -g root /etc/ssl/nac
if [ ! -s /etc/ssl/nac/public.crt ] || [ ! -s /etc/ssl/nac/public.key ]; then
    openssl req -x509 -newkey ec -pkeyopt ec_paramgen_curve:P-256 -nodes \
        -days 3650 -subj "/CN=nac.local" \
        -addext "subjectAltName=DNS:nac.local" \
        -keyout /etc/ssl/nac/public.key -out /etc/ssl/nac/public.crt
fi
chown "$PUBLIC_USER:$PUBLIC_USER" /etc/ssl/nac/public.key
chmod 600 /etc/ssl/nac/public.key
chmod 644 /etc/ssl/nac/public.crt
openssl x509 -in /etc/ssl/nac/public.crt -outform der | sha256sum | awk '{print $1}' > /etc/ssl/nac/public.pin
chmod 644 /etc/ssl/nac/public.pin

# unit 里的沙箱指令需要 systemd 232+；更老的话 unit 会加载失败。
if command -v systemctl >/dev/null 2>&1; then
    ver=$(systemctl --version | head -1 | awk '{print $2}')
    if [ "${ver:-0}" -lt 232 ] 2>/dev/null; then
        echo "!! systemd 版本 $ver 偏旧，unit 里的部分沙箱指令会加载失败：" >&2
        echo "   请删掉 nac-*.service 里标注为 232+ 的几行再装。" >&2
    fi
fi

echo "== 二进制"
install -m 0755 -o root -g root "$FROM/contentd" "$DEST/bin/contentd"
install -m 0755 -o root -g root "$FROM/publish" "$DEST/bin/publish"
install -m 0755 -o root -g root "$FROM/probe" "$DEST/bin/probe"

echo "== 页面源码"
if [ "$KEEP_SITE" = 1 ] && [ -d "$DEST/site" ] && [ -d "$DEST/admin/site" ]; then
    echo "保留现有 $DEST/site 与 $DEST/admin/site（--keep-site）"
else
    rm -rf "$DEST/site" "$DEST/admin/site"
    cp -r site "$DEST/site"
    cp -r admin/site "$DEST/admin/site"
fi
chown -R root:"$SHARE_GROUP" "$DEST/site"
chmod -R u=rwX,g=rX,o=rX "$DEST/site"
chown -R "$CONSOLE_USER:$CONSOLE_USER" "$DEST/admin"
chmod -R u=rwX,g=,o= "$DEST/admin"

# 先把现有 data/archive 交给公开用户，发布才能写 gen/current；admin 交给运营台。
chown -R "$PUBLIC_USER:$SHARE_GROUP" "$DEST/data" "$DEST/archive"
find "$DEST/data" "$DEST/archive" -type d -exec chmod 2770 {} +
find "$DEST/data" "$DEST/archive" -type f -exec chmod 660 {} +
chown -R "$CONSOLE_USER:$CONSOLE_USER" "$DEST/admin"
chmod -R u=rwX,g=,o= "$DEST/admin"

echo "== 发布两套站点"
as_user() {
    user=$1
    shift
    if command -v runuser >/dev/null 2>&1; then
        runuser -u "$user" -- sh -c 'umask 0007; exec "$@"' sh "$@"
    else
        su -s /bin/sh "$user" -c "umask 0007; exec $(printf '%s ' "$@")"
    fi
}
as_user "$PUBLIC_USER" "$DEST/bin/publish" --site "$DEST/site" --data "$DEST/data" --archive "$DEST/archive"
as_user "$CONSOLE_USER" "$DEST/bin/publish" --site "$DEST/admin/site" --data "$DEST/admin/data"

# 发布后统一权限：公开数据属 public + group share；admin 只属 console。
chown -R "$PUBLIC_USER:$SHARE_GROUP" "$DEST/data" "$DEST/archive"
find "$DEST/data" "$DEST/archive" -type d -exec chmod 2770 {} +
find "$DEST/data" "$DEST/archive" -type f -exec chmod 660 {} +
chown -R "$CONSOLE_USER:$CONSOLE_USER" "$DEST/admin"
chmod -R u=rwX,g=,o= "$DEST/admin"
chown -R root:"$SHARE_GROUP" "$DEST/site"
chmod -R u=rwX,g=rX,o=rX "$DEST/site"
chown root:root "$DEST/bin/contentd" "$DEST/bin/publish" "$DEST/bin/probe"
chmod 0755 "$DEST/bin/contentd" "$DEST/bin/publish" "$DEST/bin/probe"

echo "== systemd"
for unit in nac-contentd nac-console; do
    src="deploy/$unit.service"
    dst="/etc/systemd/system/$unit.service"
    sed -e "s#/srv/nac#$DEST#g" "$src" > "$dst"
    case "$unit" in
        nac-contentd)
            sed -i -e "s#^User=nac-public#User=$PUBLIC_USER#" \
                -e "s#^Group=nac-public#Group=$PUBLIC_USER#" \
                -e "s#^SupplementaryGroups=nac-share#SupplementaryGroups=$SHARE_GROUP#" "$dst"
            ;;
        nac-console)
            sed -i -e "s#^User=nac-console#User=$CONSOLE_USER#" \
                -e "s#^Group=nac-console#Group=$CONSOLE_USER#" \
                -e "s#^SupplementaryGroups=nac-share#SupplementaryGroups=$SHARE_GROUP#" "$dst"
            ;;
    esac
    UNITS="$UNITS $unit"
done

# 健康检查与每日备份（timer 由下面 enable）
for unit in nac-health nac-backup; do
    sed -e "s#/srv/nac#$DEST#g" "deploy/$unit.service" > "/etc/systemd/system/$unit.service"
    sed -e "s#/srv/nac#$DEST#g" "deploy/$unit.timer" > "/etc/systemd/system/$unit.timer"
done
sed -i "s#^User=nac-public#User=$PUBLIC_USER#" /etc/systemd/system/nac-health.service
install -m 0755 -o root -g root deploy/nac-backup.sh /usr/local/sbin/nac-backup

# 公开站点的绑定地址是本脚本唯一的开关：绑 0.0.0.0 就等于把 IP 交给访客。
if [ "$BIND" != "127.0.0.1" ]; then
    sed -i "s#--bind 127.0.0.1#--bind $BIND#" /etc/systemd/system/nac-contentd.service
    echo "!! 公开站点绑在 $BIND，客户端直连就能看到服务器 IP（传输层方案未定）"
fi

systemctl daemon-reload
systemctl enable $UNITS || {
    echo "!! 服务 enable 失败，先看状态：" >&2
    echo "   systemctl status nac-contentd nac-console --no-pager -l" >&2
    exit 1
}
systemctl restart $UNITS || {
    echo "!! 服务启动失败，先看日志：" >&2
    echo "   systemctl status nac-contentd nac-console --no-pager -l" >&2
    echo "   journalctl -u nac-contentd -n 30 --no-pager" >&2
    echo "   journalctl -u nac-console -n 30 --no-pager" >&2
    exit 1
}

systemctl enable --now nac-health.timer nac-backup.timer || {
    echo "!! 健康检查/备份 timer 启动失败" >&2
    echo "   systemctl status nac-health.timer nac-backup.timer --no-pager -l" >&2
    exit 1
}

echo
echo "== 完成，自检"
sleep 1
systemctl is-active nac-contentd nac-console || true
echo "-- 运行身份（公开应为 $PUBLIC_USER，运营台应为 $CONSOLE_USER）"
systemctl show -p User --value nac-contentd || true
systemctl show -p User --value nac-console || true
echo "-- 监听（应该只有 127.0.0.1 上的 7000/7001，除非你选了 --bind-public）"
ss -ltnp 2>/dev/null | grep -E ':700[01]' || true
echo "-- 取一页看看（TLS pin）"
"$DEST/bin/probe" --tls-pin-file /etc/ssl/nac/public.pin --addr 127.0.0.1:7000 health || \
    echo "（自检失败，手动检查设备日志或 pin 文件）"

echo
echo "接下来："
echo "  1) 本机隧道：ssh -N -L 17001:127.0.0.1:7001 用户@服务器  → viewer 地址栏 console/1"
echo "  2) 若要让外人访问公开站点，先决定传输层；临时直连就把 nac-contentd 改成 --bind 0.0.0.0 并开防火墙端口"
echo "  3) 访问日志：$DEST/data/access.jsonl（用 logrotate 轮转）"
