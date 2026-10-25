# 运营指南

给站主：怎么构建客户端、怎么管证书、怎么让多个站点互相认识、运营台怎么用。

---

## 1. 三种构建

`windows_gui` 是**默认特性**，所以默认不会有黑窗。

| 用途 | 命令 | 结果 |
|---|---|---|
| 分发给读者 | `cargo build --release -p viewer` | GUI 子系统，无终端，**不含 ssh 代码** |
| 自己日常用 | `cargo build --release -p viewer --features admin` | 多一个自动开 SSH 隧道的能力 |
| 排障 | `cargo build -p viewer --no-default-features --features admin` | 控制台子系统，`--routes-check` / `--selftest` 有输出 |

`admin` 特性带来的全部差别：允许客户端为标了 `ssh=用户@服务器` 的站点自己拉起
`ssh -N -L 端口:主机:端口`。分发版不编译这段代码，所以普通用户拿到 exe 没有任何
可以启动子进程的能力。

## 2. 站点表与名字

`nac.routes` 说明客户端一开始认识哪些站：

```
*nac.local  MTk5LjcuMTQwLjIxNzo3MDAw   tls
console     MTI3LjAuMC4xOjcwMDE=       plain ssh=root@example.test
```

- 第一列就是**地址栏里那一整串**：`nac.local`、`iris.out`、`EX_MYTH` 都行。
  没有共用后缀，也不分层级；客户端拿它原样查表，查不到就去问服务器
- 端点用 `viewer --routes-encode 主机:端口` 生成（base64，只是不显眼）
- 第三列标记可并列：`tls` 强制加密、`plain` 强制明文、`ssh=` 只有 admin 构建认
- 带 `*` 的是默认站
- 检查：`viewer --routes-check`

名字的合法字符是字母、数字、点、连字符、下划线。所以 `EX_MYTH` 这种在真域名系统里
非法的写法在这里完全正常 —— 它只是一串用来查表的字符。

**这只是遮掩，不是加密。** 客户端要直连就必须知道端点。

## 3. 站点注册表：加站不用重发客户端

每个 contentd 用 `--registry /srv/nac/sites.json` 说明自己认识哪些站，同时用
`--name` 说明自己叫什么。**键名就是那个站的名字**，和地址栏里写的一模一样：

```json
{
  "nac.local":      {"host": "199.7.140.217", "port": 7000, "tls": true, "title": "主站"},
  "shop.nac.local": {"host": "199.7.140.217", "port": 7002, "tls": true, "title": "小器物"},
  "iris.out":       {"host": "199.7.140.217", "port": 7003, "tls": true, "title": "iris.out"},
  "EX_MYTH":        {"host": "199.7.140.217", "port": 7004, "tls": true, "title": "EX_MYTH"}
}
```

客户端遇到不认识的名字就向**当前所在的服务器**问，服务器从这份文件回答，并回
**表里的写法** —— 所以敲 `ex_myth` 也能进，地址栏最后显示 `EX_MYTH`。
于是**新加一个站不需要重新编译、重新分发客户端**。

两种用法，同一份代码：

- **单中心**：一台机器上几个站，各自的 registry 都列全所有站
- **节点**：A 只列 A、B；B 只列 B、C。从 A 能到 B，从 B 能到 C

排障用 `probe resolve` —— 问的正是客户端问的那句话：

```bash
/srv/nac/bin/probe --tls-pin-file /etc/ssl/nac/public.pin \
    --addr 199.7.140.217:7000 resolve shop
# shop -> 199.7.140.217:7002  tls  小器物
```

注意：回答没有签名，所以服务器可以引荐任意地址。所有站点都是你自己的时候这没问题。

## 4. 证书与 pin

`deploy/install.sh` 第一次运行会生成自签证书：

```
/etc/ssl/nac/public.crt     证书
/etc/ssl/nac/public.key     私钥（属 nac-public，0600）
/etc/ssl/nac/public.pin     证书的 SHA-256（客户端认的就是它）
```

生成时用 `NAC_DOMAIN` 决定 CN / SAN（默认 `nac.local`）。一张证书服务所有站，
因为客户端是按**指纹**校验的，不是按名字，也不是按 SAN。这意味着加站不用重签、
不用重发客户端；换证书才要。

### 把 pin 编进客户端

客户端构建时按顺序找 pin：

1. 环境变量 `NAC_TLS_PIN`
2. 环境变量 `NAC_TLS_PIN_FILE` 指向的文件
3. 工作区根目录的 `nac.tls-pin`
4. crate 目录下的 `nac.tls-pin`

都没有会打印 `cargo:warning`，那份构建只能连明文站点。

```bash
export NAC_TLS_PIN="$(tr -d '\n' < /etc/ssl/nac/public.pin)"
cargo build --release -p viewer
```

**重签证书后必须重编并重发所有客户端**，否则它们会连不上。
换成真证书 + 真域名之后这条才会消失。

## 5. 单文件分发

只发一个 exe，不附带任何文件：把站点表也编进去。

```bash
export NAC_EMBED_ROUTES='*nac.local MTk5LjcuMTQwLjIxNzo3MDAw tls'
export NAC_TLS_PIN="$(tr -d '\n' < /etc/ssl/nac/public.pin)"
cargo build --release -p viewer
```

- 地址表用 `obfstr` 混淆，`strings` 扫不出来（只是不显眼）
- 嵌入式地址表**优先于**用户自己放的 `nac.routes`，所以别人劫持不了
- 分发版的站点表里不要写 `console` 那一行

本机验证：

```bash
./target/release/viewer --routes-check      # 列出解析出的站点与加密方式
./target/release/viewer --selftest          # 连一次、拉清单、取一页、取一张图
```

## 6. 运营台

运营台不监听公网，只绑 `127.0.0.1:7001`；远程访问靠 SSH 隧道。

- **分发版**：不含 ssh 代码，也够不到 7001（回环绑定），普通用户无路可进
- **管理员版**：`nac.routes` 里给运营台那一行加 `ssh=用户@服务器`，启动时自动开隧道

```
console MTI3LjAuMC4xOjcwMDE=  plain ssh=root@199.7.140.217
```

隧道端口用 `7001`（与服务器一致）。ssh 需要免密（先配 key 或 ssh-agent），
客户端以 `BatchMode=yes` 启动它，不会弹口令提示。

手动开也可以：

```bash
ssh -N -L 7001:127.0.0.1:7001 root@199.7.140.217
```

打开后地址栏输入 `console.nac.local/`（或直接输 `/`）。

## 7. 权限与沙箱

每个站一个系统用户，互不可见：

| 服务 | 用户 | 能写 |
|---|---|---|
| 公开站 `nac-contentd` | `nac-public` | 自己的 `data/state`、`data/appdata`、访问日志 |
| 运营台 `nac-console` | `nac-console` | 自己的 `data/`、公开站的 `data/`（为了能发布）、`archive/` |

公开站的 unit 里 `InaccessiblePaths=/srv/nac/admin`，所以页面脚本看不到运营台源码。
两个服务都拒绝以 root 运行（退出码 3）。

## 8. 发布

```bash
sudo -u nac-public /srv/nac/bin/publish --site /srv/nac/site --data /srv/nac/data
```

- 发布失败时**旧的一代继续服务**：新代写完才原子更新 `data/current`
- `--archive <目录>` 保留带元数据的原图（归档目录不能放在 `site/` 里）
- 常用选项：`--site` `--data` `--archive`

运营台页面里也能发布（它被授予了公开站 `data/` 的写权限）。

## 9. 加一个站的完整步骤

一条命令，可重复运行（已存在就更新它）：

```bash
cd /root/Nac
./deploy/add-site.sh shop --site site-shop --port 7002 --host 199.7.140.217
```

它做四件事，每一步都可以手工复现：

| 步骤 | 结果 |
|---|---|
| 建目录 | `/srv/nac-shop/{site,data,archive}`，权限与公开站一致（2770 / setgid） |
| 发布 | 以 `nac-public` 身份跑 `publish`，产出 `/srv/nac-shop/data/gen/<代数>` |
| 装 unit | 由 `deploy/nac-contentd-site.service` 生成 `nac-contentd-shop.service`，沙箱与主站完全相同 |
| 登记 | 往 `/srv/nac/sites.json` 加一条，客户端立刻就能认出新站 |

新站绑定的地址默认抄主站 unit 里的 `--bind`，避免"主站能连、新站连不上"。

验收：

```bash
systemctl is-active nac-contentd-shop
/srv/nac/bin/probe --tls-pin-file /etc/ssl/nac/public.pin \
    --addr 199.7.140.217:7000 resolve shop      # 主站知道 shop 在哪
/srv/nac/bin/probe --tls-pin-file /etc/ssl/nac/public.pin \
    --addr 199.7.140.217:7002 hello             # 新站自报名字
```

然后在客户端地址栏写 `shop.nac.local/`。

**每站一个进程**是有意的：两个站看不见对方的数据库、会话和文件，唯一的联系是
注册表里那几行地址。

## 10. 排障

| 症状 | 先看 |
|---|---|
| 客户端「找不到站点目录 nac.routes」 | 开发版没有配置文件；分发给别人的版本不该出现 |
| 客户端「找不到站点 X」 | 当前服务器的 `sites.json` 里有没有 X；客户端连的是不是这台服务器 |
| 客户端「无法连接到服务器」 | 端口是否通；pin 是否与服务器证书一致；`tls`/`plain` 标记对不对 |
| 页面 404 | 该代里有没有这条路径；`index.py` 是否存在于目录 |
| 页面空白 / EMPTY | `journalctl -u nac-contentd -n 30`——页面错误只写日志 |
| 静态页里算错了结果 | 是否标了 `# nac:static` 却用了会话/数据接口（现在会直接报错） |
| 运营台打不开 | 是否 admin 构建；`ssh=` 是否写了；本机 ssh 能否免密 |

## 11. 磁盘与备份

- 访问日志 `data/access.jsonl` 会一直长，记得用 logrotate 轮转
- `data/appdata/` **没有配额**：留言、上传、SQLite 都在这儿
- 备份：`nac-backup.timer` 每天 04:45 打包 `site admin/site data/appdata archive`
  到 `/root/backups/nac`，保留最近 7 份
- 健康探针：`nac-health.timer` 每 5 分钟跑一次 `probe health`