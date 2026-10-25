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

**两种 exe 不要混**：

| 文件 | 带内嵌站点表 | 能 `--site` 切站 | 用途 |
|---|---|---|---|
| `dist/NacViewer.exe` | 是 | 否 | 发给别人，单文件 |
| `target/release/viewer.exe` | 否 | 是 | 自己用，读 `nac.routes` |

崩溃时 GUI 版会弹一个标题为 `Nac` 的框（并把同样的文字写进
`%TEMP%\nac-viewer-crash.txt`），不会静默消失。

## 2. 站点表与名字

`nac.routes` 说明客户端一开始认识哪些站：

```
*nac.local  MTk5LjcuMTQwLjIxNzo3MDAw   tls
console     MTI3LjAuMC4xOjcwMDE=       plain ssh=root@example.test
```

- 第一列就是**地址栏里那一整串**：`nac.local`、`iris.out`、`cain.society` 都行。
  没有共用后缀，也不分层级；客户端拿它原样查表，查不到就去问服务器
- 端点用 `viewer --routes-encode 主机:端口` 生成（base64，只是不显眼）
- 第三列标记可并列：`tls` 强制加密、`plain` 强制明文、`ssh=` 只有 admin 构建认
- 带 `*` 的是默认站
- 检查：`viewer --routes-check`

名字的合法字符是字母、数字、点、连字符、下划线。所以 `my_site` 这种在真域名系统里
非法的写法在这里完全正常 —— 它只是一串用来查表的字符。

**这只是遮掩，不是加密。** 客户端要直连就必须知道端点。

## 3. 站点注册表：加站不用重发客户端

每个 contentd 用 `--registry /srv/nac/sites.json` 说明自己认识哪些站，同时用
`--name` 说明自己叫什么。**键名就是那个站的名字**，和地址栏里写的一模一样：

```json
{
  "nac.local":      {"host": "199.7.140.217", "port": 7000, "tls": true, "title": "主站"},
  "iris.out":       {"host": "199.7.140.217", "port": 7003, "tls": true, "title": "iris.out"},
  "web.shell":      {"host": "199.7.140.217", "port": 7005, "tls": true, "title": "web.shell"},
  "conduit.sh":     {"host": "199.7.140.217", "port": 7006, "tls": true, "title": "conduit.sh"},
  "cain.society":   {"host": "199.7.140.217", "port": 7007, "tls": true, "title": "该隐学会"}
}
```

客户端遇到不认识的名字就向**当前所在的服务器**问，服务器从这份文件回答，并回
**表里的写法** —— 所以敲 `CAIN.SOCIETY` 也能进，地址栏最后显示 `cain.society`。
于是**新加一个站不需要重新编译、重新分发客户端**。

两种用法，同一份代码：

- **单中心**：一台机器上几个站，各自的 registry 都列全所有站
- **节点**：A 只列 A、B；B 只列 B、C。从 A 能到 B，从 B 能到 C

加站用 `deploy/add-site.sh`，不要手写 unit；它会把目录、发布、unit、注册表一次做完。

排障用 `probe resolve` —— 问的正是客户端问的那句话：

```bash
/srv/nac/bin/probe --tls-pin-file /etc/ssl/nac/public.pin \
    --addr 199.7.140.217:7000 resolve iris.out
# iris.out -> 199.7.140.217:7003  tls  iris.out
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

本机验证（必须从终端运行，GUI 构建的 stdout 无处可去）：

```bash
./target/release/viewer --routes-check      # 列出解析出的站点与加密方式
./target/release/viewer --selftest          # 连一次、拉清单、取一页、取一张图
```

**再确认一遍**：内嵌表优先于用户自己放的 `nac.routes`。所以带内嵌编出来的 exe
没法用 `--site 其它站` 切站 —— 要那样用得编不带内嵌的版本（这就是
`target/release/viewer.exe` 与 `dist/NacViewer.exe` 的区别）。

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

打开后地址栏输入 `console/`（或直接输 `/`）—— 名字是 `console`，不是 `console.nac.local`。

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

### 8.1 `nac.py` 是 publish 生成的，所以服务器上那个 publish 会过期

每次发布都会把 `crates/publish/src/nac_py.tmpl` 里的模板写成该代的
`scripts/nac.py`。也就是说 **`nac.py` 里有哪些函数，由服务器上那个 `publish` 二进制决定**，
跟站点源码无关。

服务器上没有 Rust，`/srv/nac/bin/publish` 是当初拷上去的。于是会出现这种事：

```
AttributeError: module 'nac' has no attribute 'sheet'
```

意思是「站点用了新的 `nac.sheet()`，但服务器那个 publish 还是老版本，它生成的
`nac.py` 里没有这个函数」。这不是站点写错了。

两种解决办法，按顺序：

1. **在本机发布，把整代产物传上去**（服务器不用装任何东西）：

   ```powershell
   .\target\release\publish.exe --site site-conduit --data data-conduit
   $gen = (Get-Content data-conduit\current -Raw).Trim()
   tar -czf gen.tar.gz -C data-conduit\gen $gen
   scp gen.tar.gz root@服务器:/tmp/
   ```

   ```bash
   tar -xzf /tmp/gen.tar.gz -C /srv/nac-conduit.sh/data/gen/
   chown -R nac-public:nac-share /srv/nac-conduit.sh/data/gen/$gen
   printf %s $gen > /srv/nac-conduit.sh/data/current     # 这一步就是上线
   ```

2. 或者让服务器能重新编译：装 Rust，`cargo build --release -p publish`，
   再 `install -m 0755 target/release/publish /srv/nac/bin/publish`。

> 判断服务器上的 publish 是不是新的：`grep -c "def sheet" /srv/nac/data/gen/*/scripts/nac.py`
> 全是 0 就是老版本。

## 9. 加一个站的完整步骤

一条命令，可重复运行（已存在就更新它）：

```bash
cd /root/Nac
./deploy/add-site.sh iris.out --site site-iris --port 7003 --host 199.7.140.217
```

名字就是你以后在地址栏里写的那个：`nac.local`、`conduit.sh`、`iris.out`、
`cain.society` 都是合法写法。目录名会跟着变成 `/srv/nac-iris.out`。

它做四件事，每一步都可以手工复现：

| 步骤 | 结果 |
|---|---|
| 建目录 | `/srv/nac-iris.out/{site,data,archive}`，权限与公开站一致（2770 / setgid） |
| 发布 | 以 `nac-public` 身份跑 `publish`，产出 `.../data/gen/<代数>` |
| 装 unit | 由 `deploy/nac-contentd-site.service` 生成 `nac-contentd-iris.out.service` |
| 登记 | 往 `/srv/nac/sites.json` 加一条，客户端立刻就能认出新站 |

绑定地址不传就抄主站 unit 里的 `--bind`，端口不传就从 7002 往上找空的。
别忘了在防火墙上放行新端口。

> ⚠️ 新站的 `data/` 里必须先放一个**空的 `access.jsonl`**：unit 的 `ReadWritePaths`
> 指向它，文件不存在时 systemd 会以 `status=226/NAMESPACE` 拒绝启动 ——
> 报错只说 "No such file or directory"，看不出是它。
> `touch /srv/nac-<名字>/data/access.jsonl && chown nac-public:nac-share` 即可。

验收：

```bash
systemctl is-active nac-contentd-iris.out
/srv/nac/bin/probe --tls-pin-file /etc/ssl/nac/public.pin \
    --addr 199.7.140.217:7000 resolve iris.out   # 主站知道它在哪
/srv/nac/bin/probe --tls-pin-file /etc/ssl/nac/public.pin \
    --addr 199.7.140.217:7003 hello              # 新站自报名字
```

然后在客户端地址栏写 `iris.out/`。

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