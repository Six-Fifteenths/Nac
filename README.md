# Nac

一座不经过 HTTP 的小站。

页面是 Python 程序，客户端是 Rust 写的原生窗口，两者之间走一套自定义的二进制 TCP 协议。
没有 HTML，没有 JavaScript，也没有浏览器内核。

```
作者写 .py  ──publish──▶  一代不可变产物  ──contentd──▶  二进制帧  ──viewer──▶  egui 窗口
```

## 它有什么

| | |
|---|---|
| 页面 | 一个 `.py` 文件就是一页；标准库随便用，`import nac` 提供排版与数据接口 |
| 寻址 | 按路径（`/notes/`），不按页号；页号只是协议内部的东西 |
| 站点 | 多个站点各自独立进程与数据目录；客户端按名字找站，服务器回答它认识谁 |
| 客户端 | 原生窗口、标签页、跨站前进后退、按站点选择 TLS |
| 发布 | 一次发布生成一代；写完才原子切换，失败时旧代继续服务 |
| 静态化 | 标了 `# nac:static` 的页面在发布时预渲染，访问时连 Python 都不启动 |

## 快速开始（本地）

```bash
cargo build --release
./target/release/publish --site site --data data
cp nac.routes.example nac.routes        # 填上 contentd 的地址
./target/release/contentd --data data --port 7000 --bind 127.0.0.1 &
./target/release/viewer
```

窗口里应该看到「一座不经过 HTTP 的小站」，地址栏是 `nac.local/`。

## 仓库结构

| 路径 | 是什么 |
|---|---|
| `crates/protocol` | 线上格式的唯一定义处，三端共用 |
| `crates/contentd` | 内容服务：读清单、跑页面、发帧；也作为站点解析器回答「某站在哪」 |
| `crates/publish` | 发布器：校验、分配路径与页号、处理图片、生成 `nac.py` |
| `crates/viewer` | 客户端：原生窗口、标签页、路由表、TLS |
| `crates/tls-client` | 自签证书 + 指纹固定的 TLS 客户端 |
| `crates/probe` | 命令行探针，部署自检与排障用 |
| `site/` | 演示站源码 |
| `admin/site/` | 运营台源码（不进公开站） |
| `deploy/` | systemd unit、安装脚本、备份脚本 |

## 文档

| 文档 | 给谁看 |
|---|---|
| [docs/user-guide.md](docs/user-guide.md) | 拿到客户端的人：地址栏怎么用、标签页、故障怎么办 |
| [docs/authoring.md](docs/authoring.md) | 写页面的人：`nac` 函数全表、标记语法、静态页陷阱 |
| [docs/admin-guide.md](docs/admin-guide.md) | 站长：构建哪几种 exe、证书与 pin、站点注册表、运营台 |
| [docs/deploy.md](docs/deploy.md) | 部署到 Linux 服务器：安装、自检、升级、排障 |
| [docs/protocol.md](docs/protocol.md) | 协议细节：帧格式、请求类型、站点解析 |
| [PROTOCOL-DESIGN.md](PROTOCOL-DESIGN.md) | 为什么这样设计，以及以后能长成什么样 |
| [PROTOCOL-DECISIONS.md](PROTOCOL-DECISIONS.md) | 决策记录：每个取舍、代价和放弃的替代方案 |

## 构建变体

```bash
cargo build --release -p viewer                                # 分发给读者：GUI、无终端、不含 ssh 代码
cargo build --release -p viewer --features admin                # 自己用：多一个自动开 SSH 隧道的能力
cargo build -p viewer --no-default-features --features admin     # 排障：带控制台窗口
```

`windows_gui` 是默认特性，所以默认不会出现黑窗；要控制台必须显式关掉默认特性。

## 现状与边界

- **单人运营**：所有站点属于一个运营者，站点之间靠注册表互相认识。
- **传输层未定**：公开站直接暴露 TCP 端口，因此服务器 IP 对客户端可见；地址栏里的域名只是显示。
- **证书是自签的**：客户端靠 SHA-256 指纹固定，因此重签证书必须重发客户端。
- **非商用**，见 [LICENSE](LICENSE)。