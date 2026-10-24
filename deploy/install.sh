#!/bin/sh
# nac 一键部署：在仓库根目录运行，目标是 Linux 服务器（systemd）。
#
#   sudo ./deploy/install.sh                 # 装到 /srv/nac，公开站点只监听回环
#   sudo ./deploy/install.sh --bind-public   # 同上，但公开站点绑 0.0.0.0（会暴露服务器 IP）
#   sudo ./deploy/install.sh --dest /opt/nac --user nac
#
# 前提：已经 cargo build --release，或者用 --from 指定二进制所在目录。

set -eu

DEST=/srv/nac
USER_NAME=nac
BIND=127.0.0.1
FROM=target/release
UNITS=""

while [ $# -gt 0 ]; do
    case "$1" in
        --dest) DEST=$2; shift 2 ;;
        --user) USER_NAME=$2; shift 2 ;;
        --from) FROM=$2; shift 2 ;;
        --bind-public) BIND=0.0.0.0; shift ;;
        -h|--help) sed -n '2,10p' "$0"; exit 0 ;;
        *) echo "未知参数：$1" >&2; exit 2 ;;
    esac
done

[ "$(id -u)" = 0 ] || { echo "需要 root（用 sudo 跑）" >&2; exit 1; }
for bin in contentd publish; do
    [ -x "$FROM/$bin" ] || { echo "找不到 $FROM/$bin，先 cargo build --release" >&2; exit 1; }
done
[ -f site/1.py ] || { echo "请在仓库根目录运行" >&2; exit 1; }
command -v python3 >/dev/null || { echo "服务器上需要 python3" >&2; exit 1; }

echo "== 用户与目录"
id -u "$USER_NAME" >/dev/null 2>&1 || useradd -r -s /usr/sbin/nologin -d "$DEST" "$USER_NAME"
mkdir -p "$DEST/bin" "$DEST/admin" "$DEST/archive"

# unit 里的沙箱指令需要 systemd 232+；更老的话 unit 会加载失败。
if command -v systemctl >/dev/null 2>&1; then
    ver=$(systemctl --version | head -1 | awk '{print $2}')
    if [ "${ver:-0}" -lt 232 ] 2>/dev/null; then
        echo "!! systemd 版本 $ver 偏旧，unit 里的部分沙箱指令会加载失败：" >&2
        echo "   请删掉 nac-*.service 里标注为 232+ 的几行再装。" >&2
    fi
fi

echo "== 二进制"
install -m 0755 "$FROM/contentd" "$DEST/bin/contentd"
install -m 0755 "$FROM/publish" "$DEST/bin/publish"

echo "== 页面源码"
# 只同步源文件；产物由 publish 重建。
rm -rf "$DEST/site" "$DEST/admin/site"
cp -r site "$DEST/site"
cp -r admin/site "$DEST/admin/site"

chown -R "$USER_NAME:$USER_NAME" "$DEST"

echo "== 发布两套站点"
as_user() {
    if command -v runuser >/dev/null 2>&1; then
        runuser -u "$USER_NAME" -- "$@"
    else
        su -s /bin/sh -c "$(printf '%s ' "$@")" "$USER_NAME"
    fi
}
as_user "$DEST/bin/publish" --site "$DEST/site" --data "$DEST/data" --archive "$DEST/archive"
as_user "$DEST/bin/publish" --site "$DEST/admin/site" --data "$DEST/admin/data"

echo "== systemd"
for unit in nac-contentd nac-console; do
    src="deploy/$unit.service"
    dst="/etc/systemd/system/$unit.service"
    sed -e "s#/srv/nac#$DEST#g" -e "s#^User=nac#User=$USER_NAME#" \
        -e "s#^Group=nac#Group=$USER_NAME#" "$src" > "$dst"
    UNITS="$UNITS $unit"
done

# 公开站点的绑定地址是本脚本唯一的开关：绑 0.0.0.0 就等于把 IP 交给访客。
if [ "$BIND" != "127.0.0.1" ]; then
    sed -i "s#--bind 127.0.0.1#--bind $BIND#" /etc/systemd/system/nac-contentd.service
    echo "!! 公开站点绑在 $BIND，客户端直连就能看到服务器 IP（传输层方案未定）"
fi

systemctl daemon-reload
systemctl enable --now $UNITS || {
    echo "!! 服务启动失败，先看日志：" >&2
    echo "   systemctl status nac-contentd --no-pager -l" >&2
    echo "   journalctl -u nac-contentd -n 30 --no-pager" >&2
    exit 1
}

echo
echo "== 完成，自检"
sleep 1
systemctl is-active nac-contentd nac-console || true
echo "-- 运行身份（应为 $USER_NAME，不是 root）"
systemctl show -p User --value nac-contentd || true
echo "-- 监听（应该只有 127.0.0.1 上的 7000/7001，除非你选了 --bind-public）"
ss -ltnp 2>/dev/null | grep -E ':700[01]' || true
echo "-- 取一页看看"
python3 - <<'PY' || echo "（自检脚本失败，手动跑 probe 更直观）"
import socket, sys
host, port = "127.0.0.1", 7000
try:
    s = socket.create_connection((host, port), timeout=3)
    # REQ / manifest：16 字节会话 + op 0
    payload = bytes(16) + b"\x00"
    frame = (1 + len(payload)).to_bytes(4, "big") + b"\x01" + payload
    s.sendall(frame)
    data = s.recv(4096)
    print("manifest frame:", len(data), "bytes")
except Exception as exc:
    print("连接失败:", exc)
PY

echo
echo "接下来："
echo "  1) 本机隧道：ssh -N -L 7001:127.0.0.1:7001 用户@服务器  → viewer 地址栏 console/1"
echo "  2) 若要让外人访问公开站点，先决定传输层；临时直连就把 nac-contentd 改成 --bind 0.0.0.0 并开防火墙端口"
echo "  3) 访问日志：$DEST/access.jsonl（用 logrotate 轮转）"
