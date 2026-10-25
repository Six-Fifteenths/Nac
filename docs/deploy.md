# 部署与运维手册

目标环境：一台 Linux 服务器（systemd 232+），Python 3.9+。
客户端环境：Windows（当前只提供 `x86_64-pc-windows-msvc` 构建）。

---

## 1. 服务器上长什么样

```
/srv/nac/                        主站（名字 home）
├─ bin/contentd             内容服务（strip + LTO）
├─ bin/publish              发布工具（运营台的「发布」按钮会调它）
├─ bin/probe                探针：健康检查与排障
├─ site/                    主站页面
├─ admin/site/              运营台页面（同样的路径规则）
├─ data/                    主站：current / gen/ / state/ / appdata/
├─ admin/data/              运营台自己的代与会话（与公开站隔离）
├─ archive/                 原图归档（用 --archive 时才有）
├─ data/access.jsonl        访问日志（配 --log 时写）
└─ sites.json               站点注册表：客户端按名字找站时用它回答

/srv/nac-shop/                   其它站各占一个同级目录（名字见注册表）
├─ site/                    shop 的页面
├─ data/                    shop 自己的代、会话、数据库
└─ archive/
```

每个站是**一个独立进程**：自己的端口、自己的 `data/`、自己的数据库。它们互相看不见
对方的数据，唯一的联系是 `sites.json` 里那几行地址。

**可再生**：`data/gen/`、`data/state/`、`admin/data/gen/`。
**要备份**：`site/`、`admin/site/`、`data/appdata/`、`archive/`、`sites.json`，
以及每个额外站点的 `site/` 与 `data/appdata/`。

## 2. 端口与身份

| 服务 | 端口 | 绑定 | 用户 | 说明 |
|---|---|---|---|---|
| `nac-contentd` | 7000 | `127.0.0.1` 或 `0.0.0.0` | `nac-public` | 主站（`--name home`） |
| `nac-console` | 7001 | `127.0.0.1` | `nac-console` | 运营台，只能是回环 |
| `nac-contentd-<名字>` | 7002 起 | 抄主站的 `--bind` | `nac-public` | 其它站，一站一个进程 |

加站点用 `deploy/add-site.sh`；端口默认从 7002 往上找空的。

## 3. 安装

### 3.1 一条命令（推荐）

```bash
git clone git@github.com:Six-Fifteenths/Nac.git && cd Nac
cargo build --release
sudo ./deploy/install.sh
```

`install.sh` 做的事（幂等，可以反复跑）：

1. 建 `nac-share` 组与 `nac-public` / `nac-console` 两个 `nologin` 用户
2. 装 `contentd`、`publish`、`probe` 到 `<dest>/bin`
3. 生成自签证书与 pin：`/etc/ssl/nac/public.{crt,key,pin}`（CN/SAN 取 `NAC_DOMAIN`，默认 `nac.local`）
4. 同步 `site/` → `<dest>/site`、`admin/site/` → `<dest>/admin/site`（`--keep-site` 时保留现有）
5. 以各自用户发布两套站点（公开站带 `--archive`）
6. 把 `deploy/*.service` 里的 `/srv/nac` 换成 `--dest` 后装进 `/etc/systemd/system`
7. 安装健康检查/每日备份的 unit 与 timer
8. `systemctl enable` + `restart`，启动 timer
9. 自检：`is-active`、`ss -ltnp`、`probe --tls-pin-file … health`

参数：

| 参数 | 默认 | 说明 |
|---|---|---|
| `--dest` | `/srv/nac` | 安装目录 |
| `--public-user` | `nac-public` | 公开站运行用户 |
| `--console-user` | `nac-console` | 运营台用户（必须与公开用户不同）|
| `--group` | `nac-share` | 公开数据共享组 |
| `--from` | `target/release` | 二进制来源 |
| `--keep-site` | 关 | 保留现有 `site/` 与 `admin/site/`，**不从仓库覆盖** |
| `--bind-public` | 关 | 公开站改绑 `0.0.0.0`，**会暴露服务器 IP** |

> ⚠️ 不带 `--keep-site` 会用仓库里的 `site/` 覆盖服务器上的页面。
> 在服务器上改过页面的，先备份或加这个参数。

### 3.2 手工步骤（脚本看不懂时对照）

```bash
# 用户与组（每个用户一个同名组，两者都加进 nac-share）
groupadd -r nac-share
groupadd -r nac-public  && useradd -r -s /usr/sbin/nologin -M -g nac-public  -G nac-share nac-public
groupadd -r nac-console && useradd -r -s /usr/sbin/nologin -M -g nac-console -G nac-share nac-console

# 目录
install -d -m 0755 -o root -g root          /srv/nac/bin
install -d -m 0750 -o root -g nac-share     /srv/nac
install -d -m 0700 -o nac-console -g nac-console /srv/nac/admin
install -d -m 2770 -o nac-public -g nac-share    /srv/nac/data /srv/nac/data/state \
    /srv/nac/data/appdata /srv/nac/archive

# 二进制
install -m 0755 -o root -g root target/release/{contentd,publish,probe} /srv/nac/bin/

# 页面与首次发布
cp -r site /srv/nac/site && chown -R root:nac-share /srv/nac/site
cp -r admin/site /srv/nac/admin/site && chown -R nac-console:nac-console /srv/nac/admin
runuser -u nac-public  -- /srv/nac/bin/publish --site /srv/nac/site --data /srv/nac/data --archive /srv/nac/archive
runuser -u nac-console -- /srv/nac/bin/publish --site /srv/nac/admin/site --data /srv/nac/admin/data
```

## 4. 公开站要不要绑 0.0.0.0

`install.sh` 默认绑 `127.0.0.1`，也就是**外人根本连不上**——只有走 SSH 隧道或本机能访问。

要让读者直接访问，必须二选一：

```bash
# A. 改 unit（推荐：显式、看得见）
sed -i 's#--bind 127.0.0.1#--bind 0.0.0.0#' /etc/systemd/system/nac-contentd.service
systemctl daemon-reload && systemctl restart nac-contentd

# B. 或重跑安装脚本
sudo ./deploy/install.sh --bind-public
```

两种都要自己**开防火墙端口**：

```bash
ufw allow 7000/tcp        # 或 firewalld 的等价命令
```

代价：客户端直连意味着**你的服务器 IP 对每个访客可见**。
这是当前传输层的取舍，不是 bug。

## 5. 加一个站点

```bash
cd /root/Nac
./deploy/add-site.sh shop --site site-shop --port 7002 --host 199.7.140.217
```

脚本是幂等的：再跑一次就是更新那个站。它依次做：

| 步骤 | 位置 |
|---|---|
| 建目录 | `/srv/nac-shop/{site,data,archive}`，属 `nac-public:nac-share`、2770 |
| 发布 | `/srv/nac-shop/data/gen/<代数>`，由 `publish` 生成 |
| 装 unit | `/etc/systemd/system/nac-contentd-shop.service` |
| 登记 | `/srv/nac/sites.json` 里一条 |

绑定地址不传就抄主站 unit 里的 `--bind`，端口不传就从 7002 往上找空的。

`sites.json` 最终长这样：

```json
{
  "home": {"host": "199.7.140.217", "port": 7000, "tls": true, "title": "主站"},
  "shop": {"host": "199.7.140.217", "port": 7002, "tls": true, "title": "小器物"}
}
```

**主站自己也必须带 `--name` 和 `--registry`**，否则它答不出"别的站在哪"。
`install.sh` 会自动写好；手工装的话别漏了这两行：

```
    --name home \
    --registry /srv/nac/sites.json \
```

验收（三条都通才算好）：

```bash
systemctl is-active nac-contentd-shop
/srv/nac/bin/probe --tls-pin-file /etc/ssl/nac/public.pin --addr 199.7.140.217:7000 resolve shop
/srv/nac/bin/probe --tls-pin-file /etc/ssl/nac/public.pin --addr 199.7.140.217:7002 hello
```

防火墙要放行新端口（本例 7002），否则客户端拿到了地址却连不上。

## 6. 升级

```bash
cd /root/Nac
git pull
cargo build --release

# 只换二进制 + 重启，不动页面、不动数据
sudo install -m 0755 -o root -g root target/release/{contentd,publish,probe} /srv/nac/bin/
sudo systemctl restart nac-contentd nac-console
```

注意：

- **不要**在没有 `--keep-site` 的情况下重跑 `install.sh`，那会用仓库版本覆盖线上页面
- 协议有不兼容改动时（例如 v3 → v4），服务端与客户端要一起升级
- 客户端是分发的产物，**重签证书后必须重发**

## 7. 日常运维

| 事 | 怎么做 |
|---|---|
| 看访问 | `tail -f /srv/nac/data/access.jsonl`（JSONL，每行一次请求） |
| 看页面错误 | `journalctl -u nac-contentd -n 50 --no-pager`（stderr 全在这儿） |
| 手动发布 | `sudo -u nac-public /srv/nac/bin/publish --site /srv/nac/site --data /srv/nac/data` |
| 健康检查 | `systemctl status nac-health.timer`；或直接 `probe --tls-pin-file /etc/ssl/nac/public.pin --addr 127.0.0.1:7000 health` |
| 备份 | `systemctl status nac-backup.timer`；产物在 `/root/backups/nac`，保留 7 份 |
| 恢复 | 解开 tgz 覆盖 `site/ admin/site/ data/appdata/ archive/`，再跑一次 publish |

`deploy/nac-backup.sh` 打包的是 `site admin/site data/appdata archive`——
也就是**不可再生**的那部分。

## 8. 自检清单

装完或改完，按顺序确认：

```bash
systemctl is-active nac-contentd nac-console      # 都应为 active
systemctl show -p User --value nac-contentd       # 应为 nac-public
systemctl show -p User --value nac-console        # 应为 nac-console
ss -ltnp | grep -E ':700[0-9]'                    # 7001 必须只绑 127.0.0.1
probe --tls-pin-file /etc/ssl/nac/public.pin --addr 127.0.0.1:7000 health   # 应输出 ok N
probe --addr 127.0.0.1:7001 manifest               # 运营台应能列出页面
```

客户端侧：

```bash
viewer --routes-check     # 站点表解析正确（tls/plain/ssh 标记）
viewer --selftest         # protocol: 4  site: <名字>，然后能取到清单与页面
```

## 9. 磁盘

`/srv/nac` 本身很小（几 MB + 页面数据）。真正吃盘的是：

- `data/appdata/`——**没有配额**，留言板、上传、SQLite 都在这儿，需要自己盯
- `data/access.jsonl`——只增不减，配 logrotate
- 服务器上的 `target/`（编译产物）与容器镜像往往才是大头

## 10. 回滚

发布是逐代切换的，`data/gen/` 里保留着每一代：

```bash
ls -1 /srv/nac/data/gen/          # 按名字排，名字是发布时的 unix 时间
echo '<旧代名>' > /srv/nac/data/current   # 原子写请用临时文件 + mv
```

正确做法：

```bash
cd /srv/nac/data
printf '%s\n' '<旧代名>' > .current.tmp && mv .current.tmp current
```

`contentd` 每次请求都重读 `current`，所以**不用重启**。