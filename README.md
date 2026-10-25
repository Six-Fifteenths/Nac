# Nac

一座不经过 HTTP 的小站。页面是 Python 程序，客户端是 Rust 写的原生窗口，
两者之间走一套自定义的二进制 TCP 协议。没有 HTML，没有 JavaScript，没有浏览器内核。

```
你写 .py ──publish──▶ 一代不可变产物 ──contentd──▶ 二进制帧 ──viewer──▶ 原生窗口
```

一个页面就是一个普通的 Python 程序：能 `import`，能读文件，能用 SQLite，能算 2ⁿ−1 步
汉诺塔。客户端只负责把它拼出来的内容画出来。

---

## 1. 三个概念，看懂就够用了

| 概念 | 是什么 | 例子 |
|---|---|---|
| **页面** | `site/` 下的一个 `.py` 文件 | `site/about.py` → `/about` |
| **站点** | 一个 contentd 进程 + 一个数据目录 + 一个名字 | `nac.local`、`iris.out`、`EX_MYTH` |
| **地址** | 名字 + 路径，名字就是一整串字符 | `iris.out/entry/blue` |

名字没有层级、没有共用后缀：`nac.local` 和 `EX_MYTH` 地位完全一样。
客户端拿它去查表，查不到就问当前服务器。

## 2. 五分钟跑起来

需要 Rust 工具链和 Python 3。

```bash
cargo build --release                                   # 编译全部工具
./target/release/publish --site site --data data        # 发布主站
cp nac.routes.example nac.routes                        # 填上服务器地址
./target/release/contentd --data data --port 7000 --bind 127.0.0.1 &
./target/release/viewer
```

（Windows 上把后两条分别开两个终端跑 —— 行尾的 `&` 是 Unix 写法。
下面这段命令要是不顺利，[docs/start-here.md](docs/start-here.md) 里有一份带排查步骤的版本。）

窗口里应该出现「一座不经过 HTTP 的小站」，地址栏是 `nac.local/`。

`nac.routes` 一行一个站，端点用 `viewer --routes-encode 主机:端口` 生成：

```
*nac.local  MTI3LjAuMC4xOjcwMDA=   plain
```

检查这份表读得对不对：`./target/release/viewer --routes-check`。

## 3. 线上现在有四个站

同一个代码库，四份页面、四个进程、四份数据，互不可见：

| 名字 | 页面 | 是什么 |
|---|---|---|
| `nac.local` | `site/` | 主站：汉诺塔动画、Hacker News 复刻、留言板、全套排版示例 |
| `shop.nac.local` | `site-shop/` | 小器物：购物车、下单、订单、扣库存（SQLite） |
| `iris.out` | `site-iris/` | 一份清单：看过听过的东西，带分数与统计 |
| `EX_MYTH` | `site-myth/` | 八条神话，条目互相指路；名字里的下划线在真域名系统里是非法的 |
| `web.shell` | `site-web.shell/` | 终端样式的站：命令、密码门、彩蛋（照 WebShell 复刻） |
| `conduit.sh` | `site-conduit/` | 一个真的多页面应用：注册、发文、评论、标签、收藏、关注、分页（RealWorld 规格） |

加第五个站不用改客户端：

```bash
./deploy/add-site.sh 新名字 --site 页面目录 --port 7005 --host 服务器IP
```

## 4. 它有什么

| | |
|---|---|
| 页面 | 一个 `.py` 就是一页；`import nac` 提供排版与数据接口，其余标准库随便用 |
| 一页一族 | 页面可以认领 `/item/` 这样的前缀，自己从地址里取参数 |
| 素材 | `_lib/`、`_data/` 里的文件原样带过去，页面照常 `import` / `open` |
| 寻址 | 按路径（`/notes/`），不按页号；页号只是协议内部的东西 |
| 动画 | 页面给一条时间轴，客户端按关键帧插值 —— 60 帧，服务端只被请求一次 |
| 终端 | `[[term]]` 画一块深色面板：按角色上色、输入框在框里，能整页做成一个 shell |
| 站点 | 每站独立进程、独立数据目录、独立沙箱；客户端按名字找站 |
| 客户端 | 标签页、跨站前进后退、每站可选 TLS、零写盘 |
| 发布 | 一次发布生成一代；写完才原子切换，失败时旧代继续服务 |
| 静态化 | 标 `# nac:static` 的页面发布时预渲染，访问时连 Python 都不启动 |
| 出错 | 一律回空响应，原因只进服务端日志；客户端崩溃会弹框，不再静默消失 |

## 5. 仓库结构

| 路径 | 是什么 |
|---|---|
| `crates/protocol` | 线上格式的唯一定义处，三端共用 |
| `crates/contentd` | 内容服务：读清单、跑页面、发帧；也回答「某站在哪」 |
| `crates/publish` | 发布器：校验、分配路径与页号、处理图片、生成 `nac.py` |
| `crates/viewer` | 客户端：原生窗口、标签页、路由表、TLS |
| `crates/tls-client` | 自签证书 + 指纹固定的 TLS 客户端 |
| `crates/probe` | 命令行探针：部署自检与排障 |
| `site/`、`site-shop/`、`site-iris/`、`site-myth/` | 四个站的页面源码 |
| `admin/site/` | 运营台页面（不进公开站） |
| `deploy/` | 安装脚本、加站脚本、systemd unit、备份脚本 |

## 6. 构建变体

```bash
cargo build --release -p viewer                              # 分发给读者：GUI、无终端、不含 ssh 代码
cargo build --release -p viewer --features admin             # 自己用：多一个自动开 SSH 隧道的能力
cargo build -p viewer --no-default-features --features admin # 排障：带控制台窗口
```

`windows_gui` 是默认特性，所以默认没有黑窗；要控制台必须显式关掉默认特性。

单文件分发（把站点表和证书指纹都编进去）：

```powershell
# Windows PowerShell
$env:NAC_EMBED_ROUTES='*nac.local MTk5LjcuMTQwLjIxNzo3MDAw tls'
$env:NAC_TLS_PIN=(Get-Content nac.tls-pin -Raw).Trim()
cargo build --release -p viewer          # 产物：target\release\viewer.exe
```

## 7. 文档

**第一次来先看 [docs/start-here.md](docs/start-here.md)**：手把手，每一步都有
"应该看到什么"和"没看到怎么办"，不需要先理解任何概念。
**想先搞懂它是怎么运作的**，看 [docs/how-it-works.md](docs/how-it-works.md)：
从"一个 `.py` 文件"讲到"窗口里出现字"，不需要任何背景知识。

其余文档按用途分：

| 文档 | 给谁看 |
|---|---|
| [docs/start-here.md](docs/start-here.md) | **新手**：从零写出一页、看到它、传到服务器 |
| [docs/how-it-works.md](docs/how-it-works.md) | **想理解原理的人**：按 F5 到字出现，中间发生了什么 |
| [docs/user-guide.md](docs/user-guide.md) | 拿到客户端的人：地址栏怎么用、标签页、出问题怎么办 |
| [docs/authoring.md](docs/authoring.md) | 写页面的人：`nac` 函数全表、标记语法、静态页陷阱 |
| [docs/admin-guide.md](docs/admin-guide.md) | 站长：构建哪几种 exe、证书与 pin、注册表、运营台 |
| [docs/deploy.md](docs/deploy.md) | 装到 Linux 服务器：安装、加站、升级、排障 |
| [docs/protocol.md](docs/protocol.md) | 协议细节：帧格式、请求类型、站点解析 |
| [PROTOCOL-DESIGN.md](PROTOCOL-DESIGN.md) | 为什么这样设计，以及以后能长成什么样 |
| [PROTOCOL-DECISIONS.md](PROTOCOL-DECISIONS.md) | 决策记录：每个取舍、代价、放弃的替代方案 |

## 8. 边界（先说清楚）

- **单人运营**：所有站点属于一个运营者，站点之间靠注册表互相认识。
- **传输层未定**：公开站直接暴露 TCP 端口，服务器 IP 对客户端可见；地址栏里的名字只是名字。
- **证书是自签的**：客户端只认那张证书的 SHA-256，所以重签证书必须重发客户端。
- **页面代码是可信的**：页面能起子进程、读服务器文件、连外网。它挡得住"读者"，挡不住"作者"。
  要允许别人投稿，得先加隔离。
- **非商用**，见 [LICENSE](LICENSE)。
