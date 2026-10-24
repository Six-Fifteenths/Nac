# 运营指南

这份文档给站长：怎么部署、发布、看日志、备份和处理常见故障。
普通读者请看 [用户指南](user-guide.md)。

## 1. 前提

- Linux x86_64 服务器，systemd
- Python 3.9+
- `openssl`（首次安装生成 TLS 证书）
- 想自己编译时再装 Rust（1.80+）

Nac 不依赖数据库、nginx 或外部服务。一个服务进程 + 一个数据目录就能跑。

## 2. 首次部署

```bash
git clone <仓库地址> Nac
cd Nac
cargo build --release
sudo ./deploy/install.sh --bind-public
```

`--bind-public` 让公开站在 7000 对外监听；不加时只监听回环（适合先本机验证）。
脚本会：

1. 创建 `nac-share` 组、`nac-public` 与 `nac-console` 两个服务用户
2. 安装 `contentd`、`publish`、`probe`
3. 初始化 `/srv/nac` 并发布两套站点
4. 生成自签 TLS 证书到 `/etc/ssl/nac/`，并写 pin
5. 安装并启动两个服务、健康检查 timer、每日备份 timer

升级时用同样的命令；想保留服务器上改过的 `site/` 和 `admin/site/`，加 `--keep-site`。
协议 v3 的清单带路径字段，服务端与客户端需一起升级。
更细的目录/unit 说明见 [deploy.md](deploy.md)。

## 3. 两个服务

| 服务 | 用户 | 监听 | 用途 |
|---|---|---|---|
| `nac-contentd` | `nac-public` | `0.0.0.0:7000`（TLS） | 公开站 |
| `nac-console` | `nac-console` | `127.0.0.1:7001`（明文） | 运营台 |

公开站必须 TLS 才能被正式客户端连接；运营台只在回环，不需要 TLS。
公开站只能写 `data/state`、`data/appdata`、`data/access.jsonl`；
`data/gen`、`current`、`site`、`bin` 只读，`admin/` 不可见。
运营台可以写 `admin/data`、公开 `data/`、`archive/`，用于“发布”按钮。

## 4. TLS 与客户端

安装脚本生成：

- `/etc/ssl/nac/public.crt`、`public.key`
- `/etc/ssl/nac/public.pin`（证书 SHA-256）

公开客户端编译时把 pin 写进二进制，连接时只认这张证书。
**重新生成证书后必须重编客户端**，否则会连接失败。

公开客户端构建（在仓库根目录）：

```bash
export NAC_EMBED_ROUTES='*home <端点的 base64>'
export NAC_TLS_PIN="$(tr -d '\n' < /etc/ssl/nac/public.pin)"
cargo build --release --target x86_64-pc-windows-gnu -p viewer --features windows_gui
```

端点 base64：`./target/release/viewer --routes-encode 主机:端口`。
在本服务器上也可以直接用 `/root/scripts/build-nac-viewer-windows.sh --endpoint 主机:端口`。
生成的 exe 是 GUI 子系统，双击运行，不读任何 `nac.routes`。

## 5. 运营台

运营台不监听公网，用 SSH 隧道：

```bash
ssh -N -L 17001:127.0.0.1:7001 root@服务器
```

Windows 上运行同一个客户端，但加 `--plaintext` 指向隧道：

```powershell
.\NacViewer.exe --plaintext --addr 127.0.0.1:17001
```

地址栏会显示 `nac.local/`（如果用多站点 `nac.routes`，则是 `console.nac.local/`）；直接输入路径即可：

| 路径 | 页面 |
|---|---|
| `/` | 概览 |
| `/publish` | 发布新代 |
| `/data` | 看数据 / 清留言 |
| `/inventory` | 内容清单 |
| `/stats` | 访问统计 |

## 6. 发布内容

页面是 `site/` 下的 Python 文件，路径由文件名/目录决定：

| 文件 | 地址 |
|---|---|
| `site/index.py` | `/` |
| `site/about.py` | `/about` |
| `site/notes/index.py` | `/notes/` |
| `site/notes/a.py` | `/notes/a` |

也可以写 `# nac:path /custom` 覆盖；同一路径只能有一个页面。
页面之间用 `nac.link_path("/notes/", "笔记")` 链接。

```bash
./bin/publish --site site --data data --archive archive
```

或在运营台 `/publish` 点“发布”。发布前会做语法检查；失败不会切换 `current`。
发布成功后 `contentd` 立即读新代，不需要重启。

- 保留最近 5 代；回滚把 `data/current` 改成旧代名
- 图片元数据会被剥离；原图在 `archive/`
- 静态页：页面第一行附近写 `# nac:static`，发布时预渲染，运行时不再起 Python；
  静态页不要用表单/会话
- 客户端按 manifest 里的内容 hash 缓存静态页；发布后 hash 变化会自动重取，没变就不发请求

## 7. 日志、健康、备份

- 访问日志：`/srv/nac/data/access.jsonl`（JSONL，不记 IP，600/660）
  由 `/etc/logrotate.d/nac` 每周轮转 8 份
- 健康检查：`nac-health.timer` 每 5 分钟跑一次 `probe health`，失败看
  `journalctl -u nac-health.service`
- 备份：`nac-backup.timer` 每日 04:45，备份 `site`、`admin/site`、`data/appdata`、`archive`
  到 `/root/backups/nac/`，保留 7 份
- 服务日志：`journalctl -u nac-contentd -n 50`、`journalctl -u nac-console -n 50`

## 8. 中继 / 隐藏源站

公开客户端直连 7000 会暴露服务器 IP。要隐藏源站：

- 用第二台 VPS 做 **TCP 层转发**（nginx stream / HAProxy / socat），客户端嵌入中继地址；
- 源站 7000 只允许中继 IP 访问；
- 客户端到源站的 TLS 仍然端到端（中继只转发字节），pin 不用变；
- 中继如果终止 TLS，证书会变，必须重编客户端并更新 pin。

没有第二台机器时，才考虑 Cloudflare Worker/WSS 桥或内置 Tor，工作量大得多。

## 9. 常见故障

| 现象 | 先看 |
|---|---|
| 客户端连不上 | `ss -ltnp \| grep 7000`、`journalctl -u nac-contentd`、pin 是否匹配 |
| 运营台打不开 | 隧道是否在；`--plaintext` 是否加了；7001 是否监听 |
| 发布按钮失败 | 运营台能否写 `data/`；`journalctl -u nac-console` |
| 页面空白 | `journalctl -u nac-contentd` 里的 Python 报错、页面是否超时/超 16KB |
| 日志不增长 | unit 里 `--log` 路径；`data/` 是否可写；logrotate 配置 |
| 健康检查失败 | 手动 `/srv/nac/bin/probe --tls-pin-file /etc/ssl/nac/public.pin --addr 127.0.0.1:7000 health` |
| 备份没跑 | `journalctl -u nac-backup.service`；`/root/backups/nac` 权限 |

## 10. 还没做的

- Windows 代码签名：当前 exe 未签名，SmartScreen 会提示；需要证书后另行签名
- 客户端自动更新：目前换版本需要重新分发 exe
- 图片客户端缓存：图片为单连接流式传输，解码结果在本次运行内缓存；重开客户端会重新下载，暂无磁盘缓存
