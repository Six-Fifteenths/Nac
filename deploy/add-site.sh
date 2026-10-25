#!/bin/sh
# 给已经装好的 nac 增加一个站点。
#
#   sudo ./deploy/add-site.sh shop                      # 用 site-shop/ 作为页面源码
#   sudo ./deploy/add-site.sh shop --site site-shop --port 7002 --host 1.2.3.4
#
# 它做四件事：
#   1. 建 /srv/nac-<名字>/{site,data,archive}，权限与公开站一致
#   2. 用一个独立的 contentd 进程跑它（自己的端口、数据目录、沙箱）
#   3. 把名字登记进 /srv/nac/sites.json，别的服务器和客户端才知道它存在
#   4. 发布页面，启动服务，自检一次
#
# 每个站点是独立的进程，互相看不见对方的数据；能"认识"对方完全靠注册表。

set -eu

BASE=/srv/nac
NAME=
PORT=
HOST=
SITE=
BIND=
FROM=
TLS_DIR=/etc/ssl/nac

usage() {
    sed -n '2,16p' "$0"
    exit 0
}

while [ $# -gt 0 ]; do
    case "$1" in
        --port) PORT=$2; shift 2 ;;
        --host) HOST=$2; shift 2 ;;
        --site) SITE=$2; shift 2 ;;
        --bind) BIND=$2; shift 2 ;;
        --from) FROM=$2; shift 2 ;;
        -h|--help) usage ;;
        -*) echo "未知参数：$1" >&2; exit 2 ;;
        *) NAME=$1; shift ;;
    esac
done

[ -n "$NAME" ] || { echo "用法：$0 <站点名> [--port N] [--host IP]" >&2; exit 2; }

# 参数可能带着粘贴时的空白或回车（跨平台编辑最容易带进来的就是 \r）。
NAME=$(printf '%s' "$NAME" | tr -d '[:space:]')
PORT=$(printf '%s' "$PORT" | tr -d '[:space:]')
HOST=$(printf '%s' "$HOST" | tr -d '[:space:]')
BIND=$(printf '%s' "$BIND" | tr -d '[:space:]')

case "$NAME" in
    *[!A-Za-z0-9._-]*|'')
        echo "站点名只能用字母、数字、点、连字符和下划线：$NAME" >&2
        echo "（名字就是地址栏里那一整串，比如 nac.local / iris.out / EX_MYTH）" >&2
        exit 2
        ;;
esac

[ "$(id -u)" = 0 ] || { echo "需要 root" >&2; exit 1; }
[ -x "$BASE/bin/contentd" ] || { echo "先运行 install.sh：$BASE/bin/contentd 不存在" >&2; exit 1; }
[ -s "$BASE/sites.json" ] || { echo "$BASE/sites.json 不存在，先重跑 install.sh" >&2; exit 1; }

ROOT=/srv/nac-$NAME
SITE=${SITE:-site-$NAME}
[ -d "$SITE" ] || { echo "找不到页面源码目录：$SITE" >&2; exit 1; }

# 绑定地址跟公开站保持一致，免得"主站能连、新站连不上"。
if [ -z "$BIND" ]; then
    BIND=$(grep -o -- '--bind [0-9.]*' /etc/systemd/system/nac-contentd.service 2>/dev/null |
        awk '{print $2}' | head -1)
    BIND=${BIND:-127.0.0.1}
fi

# 注册表里已经有这个站就直接覆盖（重复运行是安全的）。
if python3 -c "import json,sys; sys.exit(0 if '$NAME' in json.load(open('$BASE/sites.json')) else 1)"; then
    echo "== 站点 $NAME 已存在，将更新它"
    [ -n "$PORT" ] || PORT=$(python3 -c "
import json
print(json.load(open('$BASE/sites.json'))['$NAME']['port'])")
fi

if [ -z "$HOST" ]; then
    HOST=$(python3 -c "
import json
sites = json.load(open('$BASE/sites.json'))
for entry in sites.values():
    if entry.get('host'):
        print(entry['host']); break
")
fi
[ -n "$HOST" ] || HOST=$(hostname -I 2>/dev/null | awk '{print $1}')
[ -n "$HOST" ] || HOST=127.0.0.1

if [ -z "$PORT" ]; then
    PORT=7002
    while python3 -c "
import json,sys
sites = json.load(open('$BASE/sites.json'))
sys.exit(0 if any(e.get('port') == $PORT for e in sites.values()) else 1)
" || ss -ltn 2>/dev/null | grep -q ":$PORT "; do
        PORT=$((PORT + 1))
        [ "$PORT" -lt 7100 ] || { echo "找不到空闲端口" >&2; exit 1; }
    done
fi

echo "== 站点 $NAME：$ROOT，端口 $PORT，绑定 $BIND，对外 $HOST"

install -d -m 0755 -o root -g root "$ROOT"
install -d -m 2770 -o nac-public -g nac-share \
    "$ROOT/data" "$ROOT/data/state" "$ROOT/data/appdata" "$ROOT/archive"
[ -e "$ROOT/data/access.jsonl" ] || \
    install -m 0660 -o nac-public -g nac-share /dev/null "$ROOT/data/access.jsonl"

rm -rf "$ROOT/site"
cp -r "$SITE" "$ROOT/site"
chown -R root:nac-share "$ROOT/site"
chmod -R u=rwX,g=rX,o=rX "$ROOT/site"

echo "== 发布"
chown -R nac-public:nac-share "$ROOT/data" "$ROOT/archive"
find "$ROOT/data" "$ROOT/archive" -type d -exec chmod 2770 {} +
find "$ROOT/data" "$ROOT/archive" -type f -exec chmod 660 {} +
if command -v runuser >/dev/null 2>&1; then
    runuser -u nac-public -- sh -c 'umask 0007; exec "$@"' sh \
        "$BASE/bin/publish" --site "$ROOT/site" --data "$ROOT/data" --archive "$ROOT/archive"
else
    su -s /bin/sh nac-public -c "umask 0007; exec $BASE/bin/publish --site $ROOT/site --data $ROOT/data --archive $ROOT/archive"
fi
chown -R nac-public:nac-share "$ROOT/data" "$ROOT/archive"
find "$ROOT/data" "$ROOT/archive" -type d -exec chmod 2770 {} +
find "$ROOT/data" "$ROOT/archive" -type f -exec chmod 660 {} +

echo "== 登记进注册表"
python3 - "$BASE/sites.json" "$NAME" "$HOST" "$PORT" <<'PY'
import json
import os
import sys

path, name, host, port = sys.argv[1], sys.argv[2], sys.argv[3], int(sys.argv[4])
with open(path, encoding="utf-8") as handle:
    sites = json.load(handle)
sites[name] = {"host": host, "port": port, "tls": True, "title": name}
tmp = path + ".tmp"
with open(tmp, "w", encoding="utf-8") as handle:
    json.dump(sites, handle, ensure_ascii=False, indent=2, sort_keys=True)
    handle.write("\n")
os.replace(tmp, path)
PY
chown root:nac-share "$BASE/sites.json"
chmod 0644 "$BASE/sites.json"

echo "== systemd"
UNIT=nac-contentd-$NAME
sed -e "s#@NAME@#$NAME#g" \
    -e "s#@PORT@#$PORT#g" \
    -e "s#@BIND@#$BIND#g" \
    -e "s#@DATA@#$ROOT/data#g" \
    "$(dirname "$0")/nac-contentd-site.service" > "/etc/systemd/system/$UNIT.service"
systemctl daemon-reload
systemctl enable --now "$UNIT" >/dev/null 2>&1 || systemctl restart "$UNIT"

# 主站自己也要有名字和注册表，否则它回答不了"shop 在哪"。
if ! grep -q -- '--registry' /etc/systemd/system/nac-contentd.service; then
    echo "!! 主站 unit 没有 --registry，客户端问不出 $NAME 在哪。" >&2
    echo "   请重跑 install.sh，或手工给 nac-contentd 加上 --name/--registry 后重启。" >&2
fi

echo "== 自检"
sleep 1
"$BASE/bin/probe" --tls-pin-file "$TLS_DIR/public.pin" --addr "127.0.0.1:$PORT" hello ||
    echo "（自检失败：看 journalctl -u $UNIT）"

echo
echo "完成。客户端地址栏：$NAME.<后缀>/"
echo "注册表：$BASE/sites.json"
echo "日志：journalctl -u $UNIT -f"
