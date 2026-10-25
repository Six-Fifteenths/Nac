# 部署与运维

目标：一台 Linux 服务器（systemd），从零到四个站在跑。

每一步后面都写了**应该看到什么**，对不上就停在那一节排查。

---

## 1. 服务器上长什么样

```
/srv/nac/                      主站，名字 nac.local
├─ bin/contentd                内容服务
├─ bin/publish                 发布工具（运营台的「发布」按钮会调它）
├─ bin/probe                   探针：自检与排障
├─ site/                       主站页面源码
├─ data/                       主站数据：current / gen/ / state/ / appdata/
├─ admin/{site,data}/          运营台（自己的站点与数据，与公开站隔离）
├─ archive/                    原图归档（用 --archive 时才有）
└─ sites.json                  站点注册表：客户端按名字找站时照它回答

/srv/nac-shop.nac.local/       其它站各占一个同级目录，名字见注册表
/srv/nac-iris.out/             ├─ site/     页面源码
/srv/nac-EX_MYTH/              ├─ data/     自己的代、会话、数据库
                               └─ archive/
```

**每站一个进程**：自己的端口、自己的 `data/`、自己的数据库、自己的沙箱。
它们互相看不见对方的数据，唯一的联系是 `sites.json` 里那几行地址。

**可再生**：`data/gen/`、`data/state/`、各站的 `data/gen/`。
**要备份**：各站的 `site/`、`data/appdata/`、`archive/`，以及 `sites.json`。

## 2. 端口与身份

| 服务 | 端口 | 绑定 | 用户 | 是什么 |
|---|---|---|---|---|
| `nac-contentd` | 7000 | `0.0.0.0` 或 `127.0.0.1` | `nac-public` | 主站（`--name nac.local`） |
| `nac-console` | 7001 | 只 `127.0.0.1` | `nac-console` | 运营台 |
| `nac-contentd-<名字>` | 7002 起 | 抄主站的 `--bind` | `nac-public` | 其它站，一站一个进程 |

## 3. 装

### 3.1 一条命令（推荐）

```bash
# 先把二进制编出来（服务器上需要 gcc 和 rust 工具链）
cargo build --release

sudo ./deploy/install.sh                 # 公开站只绑回环
sudo ./deploy/install.sh --bind-public   # 公开站绑 0.0.0.0（会暴露服务器 IP）
```

它会：建用户组与目录 → 生成自签证书与 pin → 装二进制 → 发布主站与运营台
→ 写 `sites.json` → 装 systemd 单元 → 自检。

**应该看到**：最后几行是 `systemctl is-active` 显示 `active`，
`ss -ltnp` 里有 `127.0.0.1:7000` 和 `127.0.0.1:7001`，`probe` 自检通过。

### 3.2 手工步骤（脚本看不懂时对照）

```bash
# 用户与组：每个用户一个同名组，两者都加进 nac-share
groupadd -r nac-public; useradd -r -s /usr/sbin/nologin -g nac-public -G nac-share nac-public

# 目录
install -d -o root -g nac-share -m 0750 /srv/nac
install -d -o root -g root -m 0755 /srv/nac/bin
install -d -o nac-public -g nac-share -m 2770 /srv/nac/{data,data/state,data/appdata,archive}

# 证书与 pin（一张证书服务所有站）
openssl req -x509 -newkey ec -pkeyopt ec_paramgen_curve:P-256 -nodes -days 3650 \
    -subj "/CN=nac.local" -addext "subjectAltName=DNS:nac.local" \
    -keyout /etc/ssl/nac/public.key -out /etc/ssl/nac/public.crt
openssl x509 -in /etc/ssl/nac/public.crt -outform der | sha256sum | awk '{print $1}' \
    > /etc/ssl/nac/public.pin

# 二进制 + 源码 + 首次发布
install -m 0755 target/release/{contentd,publish,probe} /srv/nac/bin/
cp -r site /srv/nac/site
runuser -u nac-public -- /srv/nac/bin/publish --site /srv/nac/site --data /srv/nac/data
```

## 4. 公开站要不要绑 0.0.0.0

不绑就只有本机能连（配合 SSH 隧道用）。绑了客户端才能直连，但服务器 IP 一定暴露。

```bash
sed -i 's#--bind 127.0.0.1#--bind 0.0.0.0#' /etc/systemd/system/nac-contentd.service
systemctl daemon-reload && systemctl restart nac-contentd
```

## 5. 加一个站

```bash
cd /root/Nac
./deploy/add-site.sh iris.out --site site-iris --port 7003 --host 199.7.140.217
```

脚本是幂等的：再跑一次就是更新那个站。它依次做四件事：

| 步骤 | 位置 |
|---|---|
| 建目录 | `/srv/nac-iris.out/{site,data,archive}`，属 `nac-public:nac-share`、2770 |
| 发布 | `/srv/nac-iris.out/data/gen/<代数>` |
| 装 unit | `/etc/systemd/system/nac-contentd-iris.out.service` |
| 登记 | `/srv/nac/sites.json` 里一条 |

绑定地址不传就抄主站 unit 里的 `--bind`，端口不传就从 7002 往上找空的。
名字允许字母、数字、点、连字符、下划线 —— `EX_MYTH` 这种也行。

**应该看到**：`自检` 那行打印 `version 5  site iris.out`。

`sites.json` 最终长这样：

```json
{
  "nac.local":      {"host": "199.7.140.217", "port": 7000, "tls": true, "title": "主站"},
  "shop.nac.local": {"host": "199.7.140.217", "port": 7002, "tls": true, "title": "小器物"},
  "iris.out":       {"host": "199.7.140.217", "port": 7003, "tls": true, "title": "iris.out"},
  "EX_MYTH":        {"host": "199.7.140.217", "port": 7004, "tls": true, "title": "EX_MYTH"}
}
```

⚠️ **主站自己也必须带 `--name` 和 `--registry`**，否则它答不出"别的站在哪"。
`install.sh` 会自动写好；手工装的话别漏：

```
    --name nac.local \
    --registry /srv/nac/sites.json \
```

⚠️ **防火墙要放行新端口**（本例 7003），否则客户端拿到了地址却连不上。

## 6. 升级

```bash
cd /root/Nac && git pull
cargo build --release
install -m 0755 target/release/{contentd,publish,probe} /srv/nac/bin/
runuser -u nac-public -- /srv/nac/bin/publish --site /srv/nac/site --data /srv/nac/data
systemctl restart nac-contentd
```

- 只改页面：**不用**换二进制，跑一次 `publish` 即可（发布完立刻生效，旧代还在磁盘上）
- 改协议（版本号变了）：服务端和客户端必须一起升，否则一律回 `EMPTY`
- 加站后想让新站也吃到新二进制：把上面那行 `install` 再跑一次，然后
  `systemctl restart 'nac-contentd*'`

**服务器上没装编译器**会看到 `failed to find tool "cc"`。装一下：

```bash
apt-get install -y gcc libc6-dev
```

## 7. 日常运维

```bash
systemctl status nac-contentd                # 主站状态
journalctl -u nac-contentd -n 50             # 页面报错都在这（print 也在）
systemctl restart nac-contentd               # 改完配置重启
tail -f /srv/nac/data/access.jsonl           # 访问日志（一行一个 JSON）
```

| 想做什么 | 怎么做 |
|---|---|
| 看某页为什么空 | `journalctl -u nac-contentd -n 50`，找 `[page 65206]` 那几行 |
| 确认证书没换 | `cat /etc/ssl/nac/public.pin`，与客户端构建时用的比 |
| 确认某站被认识 | `probe --tls-pin-file /etc/ssl/nac/public.pin --addr IP:7000 resolve 名字` |
| 确认某站在跑 | `probe --tls-pin-file /etc/ssl/nac/public.pin --addr IP:端口 hello` |

## 8. 自检清单

部署完、每次改动后，按顺序过一遍：

```bash
systemctl is-active nac-contentd nac-console 'nac-contentd-*'
ss -ltn | grep -E ':700[0-9]'
/srv/nac/bin/probe --tls-pin-file /etc/ssl/nac/public.pin --addr 127.0.0.1:7000 health   # ok N
timeout 3 openssl s_client -connect 199.7.140.217:7000 -servername nac.local </dev/null \
    2>/dev/null | grep -c "BEGIN CERTIFICATE"                                            # TLS 握手通（输出 1）
```

客户端那边：地址栏写 `nac.local/`，再写一个别的站的名字，两个都要能打开。

## 9. 磁盘

占地方的主要是 `data/gen/`（每一代都复制全部素材）。默认保留 5 代，
`publish` 会自动清旧的。图片多的时候 `archive/` 也会长起来。

```bash
du -sh /srv/nac*/data/gen /srv/nac/archive 2>/dev/null
```

## 10. 回滚

```bash
ls /srv/nac/data/gen/                 # 看有哪些代
echo <旧代数> > /srv/nac/data/current # 指回去
systemctl restart nac-contentd
```

`current` 是个纯文本文件，写哪一代就服务哪一代；写错了重启回空响应，
改回来即可。二进制回滚就是换回上一个 `contentd`。
