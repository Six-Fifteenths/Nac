# 部署与运维手册

目标环境：一台 Linux 服务器（systemd），Python 3.9+。

---

## 1. 服务器上长什么样

```
/srv/nac/
├─ bin/contentd              release 构建（strip + LTO + panic=abort）
├─ bin/publish               发布工具（运营台的"发布"按钮会调它）
├─ site/                     公开站点的页面源码
├─ admin/site/               运营台页面源码
├─ data/                     公开站点：current / gen/ / state/ / appdata/
├─ admin/data/               运营台自己的代与会话（与公开站点隔离）
├─ archive/                  原图归档（只有用 --archive 时才有）
└─ data/access.jsonl         访问日志（只有给 --log 时才写）
```

`data/gen/` 与 `data/state/` 可再生；`site/`、`admin/site/`、`data/appdata/`、`archive/` 要备份。

---

## 2. 安装

### 2.1 一条命令（推荐）

```bash
git clone git@github.com:Six-Fifteenths/Nac.git && cd Nac
cargo build --release          # 或在有 Rust 的机器上构建后传 target/release/
sudo ./deploy/install.sh
```

`install.sh` 做的事（幂等，可以反复跑）：

1. 建 `nac-share` 组与 `nac-public` / `nac-console` 两个 `nologin` 用户（公开数据组共享）
2. 装 `contentd`、`publish` 到 `/srv/nac/bin`
3. 同步 `site/` → `/srv/nac/site`、`admin/site/` → `/srv/nac/admin/site`（`--keep-site` 时保留现有）
4. 以各自用户发布两套站点（公开站 `--archive`）
5. 把 `deploy/*.service` 里的 `/srv/nac` 替换成 `--dest` 指定目录后装进 `/etc/systemd/system`
6. `systemctl enable` + `restart nac-contentd nac-console`
7. 自检：`is-active`、`ss -ltnp`、用一个裸帧向 7000 取一次 manifest

参数：

| 参数 | 默认 | 说明 |
|---|---|---|
| `--dest` | `/srv/nac` | 安装目录 |
| `--public-user` | `nac-public` | 公开站运行用户 |
| `--console-user` | `nac-console` | 运营台运行用户（必须不同于公开用户）|
| `--group` | `nac-share` | 公开数据共享组 |
| `--from` | `target/release` | 二进制来源目录 |
| `--keep-site` | 关 | 保留现有 `site/` 与 `admin/site/`，不从仓库覆盖 |
| `--bind-public` | 关 | 公开站点改绑 `0.0.0.0`，**会暴露服务器 IP** |

### 2.2 手工步骤（脚本看不懂时对照）

手工步骤与 `install.sh` 一致：创建 `nac-share`、`nac-public`、`nac-console`，按 §5.0
的权限表设置目录，分别用对应用户执行 `publish`，再安装两个 unit。步骤较多，建议直接用
`install.sh`，避免漏掉属主/组/umask。

---

## 3. 两个服务

| unit | 端口 | 绑定 | 用途 |
|---|---|---|---|
| `nac-contentd.service` | 7000 | `127.0.0.1` | 公开站点，带 `--log data/access.jsonl` 与 `--session-ttl-hours 168` |
| `nac-console.service` | 7001 | `127.0.0.1` | 运营台，额外传 `NAC_ROOT`、`NAC_LOG` |

两个 unit 都带 `Restart=always`、`NoNewPrivileges=yes`、`ProtectSystem=strict`、
`ProtectHome=yes`、`PrivateTmp=yes`、`ProtectProc=invisible`、`ProcSubset=pid`、
`SystemCallFilter=~@debug`。

**默认只绑回环，所以装完外人是连不上的。** 这是"不把 IP 交出去"的当前状态：

- 想让普通用户访问 → 必须先定传输层（Tor / 自建中继），**当前未实现**；
- 临时直连演示 → `sudo ./deploy/install.sh --bind-public`，再在防火墙放行 7000。
  这时任何抓到连接的人都知道服务器 IP，`nac.routes` 的遮掩也失效。

改完 unit 记得：

```bash
sudo systemctl daemon-reload && sudo systemctl restart nac-contentd
```

---

## 4. 运营台怎么远程用

服务器上运营台只监听 `127.0.0.1:7001`，用 SSH 隧道拉到自己电脑：

```bash
ssh -N -L 7001:127.0.0.1:7001 用户@服务器
```

固化到 `~/.ssh/config`：

```
Host nac
  HostName 服务器
  User 用户
  LocalForward 7001 127.0.0.1:7001
  ServerAliveInterval 30
```

本机客户端（`viewer`）的 `nac.routes` 加一行，用 `viewer --routes-encode 127.0.0.1:7001`
生成那条 base64：

```
*home   MTI3LjAuMC4xOjcwMDA=
console MTI3LjAuMC4xOjcwMDE=
```

然后在客户端地址栏输 `console/1`。

注意事项：

- **不要写 `-R`**（那是反向转发）。`-L` 才是"我这边 → 服务器"。
- **不要加 `-g`**，那会让本机端口对局域网开放。
- 每个请求一条新 SSH 通道；图片按 16 KB 分片，多图会多次往返。
- 隧道断了客户端显示"无法连接"，重连即恢复。

运营台没有登录，判断依据是单人 VPS：`127.0.0.1` 是网络位置而不是认证，
能在这台机器上执行命令的人本来就能直接改 `data/`，再加口令只是重复 SSH 的工作，
还会把明文口令留在磁盘上。**如果将来变成多人共享、或要绑非回环地址，就必须重新考虑**，
届时优先选"按需启动"或改 Unix socket，而不是加明文口令。

---

## 5. 上线清单

### 5.0 运行身份（最重要的一条）

页面脚本由 `contentd` 启动，**继承它的权限**。用 root 跑，等于任何页面里的一个 bug
都直接拥有 root。因此：

- `contentd` 检测到真实 UID 是 0 时**直接退出**（退出码 3），不再只是警告；
  确实需要时才加 `--allow-root`。
- 两个 unit 分别写 `User=nac-public`（公开）与 `User=nac-console`（运营台），安装后这样确认：

  ```bash
  systemctl show -p User --value nac-contentd   # 应为 nac-public
  systemctl show -p User --value nac-console    # 应为 nac-console
  ps -o user= -C contentd                        # 两行，分别是 nac-public / nac-console
  ```

- 两个用户都是 `nologin`、无 sudo、无 SSH key；共享数据通过 `nac-share` 组。

第二层是 unit 里的沙箱（两家共用）：

| 指令 | 作用 |
|---|---|
| `NoNewPrivileges=yes` | 禁止任何 setuid 提权 |
| `CapabilityBoundingSet=` / `AmbientCapabilities=` | 丢掉全部 capability |
| `RestrictSUIDSGID=yes` | 不能创建 setuid/setgid 文件 |
| `RestrictNamespaces=yes` / `LockPersonality=yes` | 关掉常见逃逸面 |
| `ProtectSystem=strict` | 整个文件系统只读，只有下面列出的目录可写 |
| `ProtectHome=yes` / `PrivateTmp=yes` / `PrivateDevices=yes` | 看不到家目录，私有 `/tmp`，无设备节点 |
| `ProtectKernelTunables/Modules/ControlGroups=yes` | 内核与 cgroup 不可写 |
| `UMask=0007` | 新文件属主/组可读写，其他用户不可读（运营台需组读）|

**两个服务各自能碰的目录**（这是"一次部署只给一个局部文件夹"的具体落地）：

| | `nac-contentd`（公开，7000，`nac-public`） | `nac-console`（运营台，7001，`nac-console`） |
|---|---|---|
| 可写 | `data/state/`、`data/appdata/`、`data/access.jsonl` | `admin/data/`、`data/`、`archive/` |
| 只读 | `bin/`、`site/`、`data/gen/`、`data/current` | `bin/`、`site/`、`admin/site/` |
| 完全不可见 | **`admin/`**（`InaccessiblePaths` + 不同 UID） | — |

几点值得注意：

- 公开站点的页面脚本**读不到**运营台的源码与数据（不是"不能写"，是看不到）；现在两个服务是不同 UID，`/proc/<pid>/root` 也跨不过去。
- 两边都**写不了** `site/` 与 `admin/site/`：页面脚本改不动内容源文件。改内容走 `git pull` 或你自己 ssh 编辑，然后 `publish`。
- `data/access.jsonl` 是公开站唯一的单文件可写例外；`data/gen/` 与 `data/current` 对公开服务只读，发布/回滚只能由运营台或手工 publish。
- 运营台必须能写 `data/`，否则"发布"按钮失效。它因此比公开站点多一份权限——这是**运营台只监听回环**的原因。

从旧版本升级上来时，`/srv/nac/access.jsonl` 会留在原地不再写入；确认新日志正常后可以删掉它。

这些指令大多需要 systemd 232+（CentOS 8 / Debian 10 / Ubuntu 20.04 起）。`ProtectProc` /
`ProcSubset` 需要 systemd 247+；当前部署为 systemd 257，已启用。更老的系统上 unit 会加载失败，
`install.sh` 会检测版本并提示。`ProtectHostname` / `ProtectClock` 仍作为可选注释保留。

注意：`InaccessiblePaths` 只隐藏直接路径；`ProtectProc` 只能对**其他用户**隐藏进程。当前已把公开站与运营台拆成不同 UID，`/proc/<pid>/root` 跨用户访问被内核拒绝（实测）；`SystemCallFilter=~@debug` 再挡 ptrace/process_vm。保留 `InaccessiblePaths` 作为直接路径防护。

### 5.1 逐项检查

服务器上：

1. `systemctl is-active nac-contentd nac-console` → 两个 `active`
2. `ss -ltnp | grep 700` → 只有 `127.0.0.1:7000`、`127.0.0.1:7001`（用了 `--bind-public` 才有 `0.0.0.0:7000`）
3. `journalctl -u nac-contentd -n 20` → 没有反复重启、没有 `bind` 失败
4. `cd /srv/nac && bin/publish --site site --data data` → `published <代> (N pages, M images)`
   （注意：以 `nac` 身份跑，`sudo -u nac ...`，否则产物属主会变成 root）
5. `tail -f /srv/nac/data/access.jsonl` → 取一次页，应出现一行 JSON
6. firewall：只留 SSH。传输层未定之前不开 7000

自己电脑：

7. `ssh -N -L 7001:127.0.0.1:7001 用户@服务器`
8. `viewer --routes-check` → 应显示 `home` 与 `console`
9. `console/1` 看概览、`console/5` 看访问统计（应能看到第 5 步产生的记录）
10. 在运营台第 2 页点"发布"→ 公开站点立刻更新（**不需要重启服务**）

---

## 6. 日常操作

| 想做的事 | 做法 |
|---|---|
| 改内容 | 改 `site/*.py` → `sudo -u nac /srv/nac/bin/publish --site /srv/nac/site --data /srv/nac/data` |
| 改完立刻生效吗 | 是。`contentd` 每次请求都重读 `data/current`，不用重启 |
| 回滚 | 把 `data/current` 里的代名改成旧代（`data/gen/` 保留最近 5 代） |
| 改运营台 | 改 `admin/site/*.py` → `publish --site admin/site --data admin/data` |
| 看实时访问 | 运营台第 5 页，或 `tail -f data/access.jsonl` |
| 清空所有数据 | 停服务 → 删 `data/appdata/` → 起服务 |
| 重启 | `sudo systemctl restart nac-contentd` |

---

## 7. 日志、清理、备份

**访问日志**（`--log`）没有内建轮转，加一个 logrotate：

```
/srv/nac/data/access.jsonl {
    weekly
    rotate 8
    compress
    missingok
    notifempty
    copytruncate
}
```

**会话文件**：`--session-ttl-hours`（默认 168 小时），后台线程启动时与每 10 分钟清一次；
`data/state/tmp/` 里崩溃残留的临时会话 1 小时后清掉。不需要手工维护。

**代目录**：`publish` 自动保留最近 5 代，更早的自动删除。

**备份**（只需这四样，其余可再生）：

```bash
tar czf nac-$(date +%F).tgz \
    -C /srv/nac site admin/site data/appdata archive
```

---

## 8. 升级

```bash
cd Nac && git pull
cargo build --release
sudo ./deploy/install.sh        # 会重新装二进制、同步源码、重新发布、重启服务
```

`install.sh` 不会删 `data/`、`admin/data/`、`archive/`，所以会话、留言、归档都留着。

---

## 9. 排障

| 症状 | 先看什么 |
|---|---|
| 客户端"无法连接到服务器" | `systemctl is-active nac-contentd`；`ss -ltnp \| grep 7000`；`--bind` 是不是回环 |
| 某页始终空白 | `journalctl -u nac-contentd -n 50` 有 Python 报错；页面是否超时 2 秒、输出超 128 KB、页面超 16 KB |
| 改了内容没生效 | 有没有重新 `publish`；`cat data/current` 是不是新代；运营台第 1 页显示当前代 |
| 发布报"页号被用了两次" | `data` 或 `archive` 目录落在 `site` 里面 |
| 发布报 `SyntaxError` | 就是那一页的 Python 语法错，按提示的行号改 |
| 图片不显示 | `nac.img()` 路径要和 `site/` 下一致；`journalctl` 里会有 `KeyError` |
| 运营台打不开 | 隧道是否建好；`nac.routes` 里有没有 `console`；7001 是否 active |
| 日志不增长 | `contentd` 启动时有没有给 `--log`；unit 里路径是否可写（属主 `nac-public`） |
| 会话文件暴涨 | 检查 `--session-ttl-hours`；每个新客户端一个文件，属正常 |

定位 Python 页面问题最直接的方式：在服务器上手动跑一次

```bash
cd /srv/nac/data/gen/$(cat /srv/nac/data/current)/scripts
sudo -u nac NAC_DATA_DIR=/tmp/nacprobe NAC_SESSION_FILE=/tmp/nacprobe.json \
    NAC_INPUT='{"page":7,"fields":{}}' python3 -B 7.py
```

输出就是页面内容；报错直接看栈。
