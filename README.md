# Nac

一套自成一体的发布平台：自己的 TCP 协议、自己的页面格式、自己的客户端。
内容不是 HTML/Markdown，每个页面是一个 Python 程序，客户端是一个独立的小窗口。

- 服务端：`contentd`，一请求一响应，页面超时 2 秒、输出上限 128 KB
- 发布工具：`publish`，检查语法、剥图片元数据、原子切换、保留 5 代
- 客户端：`viewer`，地址栏显示域名 + 路径（如 `nac.local/about`），不暴露服务器地址
- 管理端：第二个 `contentd` 站点，只监听回环，通过 SSH 隧道访问
- 页面路径：`site/index.py` → `/`，`site/notes/a.py` → `/notes/a`，目录首页用 `index.py`

## 文档

| 文档 | 给谁看 |
|---|---|
| [用户指南](docs/user-guide.md) | 普通读者：下载、双击、地址、表单、常见问题 |
| [运营指南](docs/admin-guide.md) | 站长：部署、发布、TLS、权限、备份、故障 |
| [页面编写](docs/authoring.md) | 写页面的人：`nac` 函数、标记、表单、静态页 |
| [协议](docs/protocol.md) | 想改协议的人：帧、类型、会话、错误 |
| [部署细节](docs/deploy.md) | 运维：目录、systemd、日志、排障 |
| [设计记录](PROTOCOL-DESIGN.md) | 历史取舍与实现规格 |

## 30 秒了解

```bash
cargo build --release
cp nac.routes.example nac.routes
./target/release/publish --site site --data data
./target/release/contentd --data data --port 7000 &
./target/release/viewer
```

窗口里应该看到“一座不经过 HTTP 的小站”，地址栏是 `nac.local/`。详细步骤见运营指南。

`nac.routes` 每行第三列说明这条连接怎么加密：

```
*home    MTk5LjcuMTQwLjIxNzo3MDAw   tls     # 公网，自签证书 + pin
console  MTI3LjAuMC4xOjcwMDE=       plain   # 运营台，走 SSH 隧道，回环明文
```

`viewer --routes-check` 会列出每个站点的 `auto` / `tls` / `plain`。

## 状态

| 部分 | 状态 |
|---|---|
| protocol / viewer / contentd / publish / probe | 可用，协议 v3：版本握手、清单路径、图片流式分片 |
| TLS（公开站） | 自签证书 + 客户端固定；管理端仍为回环明文。加密按站点声明（`nac.routes` 第三列 `tls` / `plain`），一个客户端可同时连两边 |
| 发布 | 原子切换 `current`；路径路由、目录首页、静态页预渲染 |
| 运维 | systemd 沙箱、分用户、资源上限、健康检查、每日备份 |
| 客户端构建 | 分发版（`windows_gui`，无 ssh 能力）／管理员版（`admin`，可自动开 SSH 隧道，GUI 或控制台两种） |
| 测试 | `cargo test --workspace` |

## 许可

仅限非商业用途，见 [LICENSE](LICENSE)。
