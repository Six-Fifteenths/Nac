# Nac

一套自成一体的发布平台：**自己的传输协议、自己的页面格式、自己的站点目录、自己的客户端**。
内容不是 HTML，也不是 Markdown——**每个页面就是一个 Python 程序**。

一个站点在一台服务器上，客户端是一个独立的小窗口程序；地址栏里没有 IP，只有 `/页码`。

```
        ┌──────────────┐        自定义二进制协议            ┌──────────────┐
        │   viewer     │  ───────────────────────────▶    │   contentd   │
        │  (egui GUI)  │   TCP · 一请求一响应 · 无 HTTP      │  (Rust 服务)  │
        └──────────────┘                                  └───────┬──────┘
               ▲                                                  │ 每次请求
               │ 站点目录 nac.routes（名字 → 端点）                 ▼
        ┌──────┴───────┐                                  ┌──────────────┐
        │ 你本机的配置  │                                   │ python3 xx.py│
        └──────────────┘                                   └──────────────┘
```

---

## 目录

- [状态](#状态)
- [它是什么，不是什么](#它是什么不是什么)
- [30 秒上手](#30-秒上手)
- [写一个页面](#写一个页面)
- [发布](#发布)
- [跑服务端](#跑服务端)
- [用客户端访问](#用客户端访问)
- [运营台](#运营台我自己的管理界面)
- [部署到服务器](#部署到服务器)
- [协议速览](#协议速览)
- [安全模型](#安全模型做到了什么没做什么)
- [已知限制](#已知限制)
- [仓库结构](#仓库结构)
- [文档](#文档)
- [许可](#许可)

---

## 状态

| 部分 | 状态 |
|---|---|
| `protocol`：帧编解码 | ✅ 往返 / 截断测试 |
| `viewer`：图形客户端 | ✅ 地址栏、前进后退、刷新、站点索引、分栏、图片、表单、异步取页 |
| `contentd`：内容服务 | ✅ Python 页、会话、图片分片、统一错误、访问日志、会话过期清理 |
| `publish`：发布工具 | ✅ 只收 `.py`、语法检查、无损剥离图片元数据、原子切换、保留 5 代 |
| `probe`：命令行冒烟客户端 | ✅ 清单 / 页 / 图 / 提交 |
| 运营台 | ✅ 第二个站点，`ssh -L` 远程访问 |
| 部署 | ✅ `deploy/install.sh` + 两个 systemd unit |
| 传输层匿名性 | ⬜ **未做**：直连即暴露服务器 IP |

测试：`cargo test --workspace`（23 项）。

---

## 它是什么，不是什么

**是**：

- 一个自有协议的内容分发系统：客户端与服务端都是本仓库的 Rust 程序，线上格式定义在 `crates/protocol/src/lib.rs`。
- 一个"页面即程序"的发布平台：页面是 Python 脚本，拼出有限几种排版块。
- 一个可以装在一台 VPS 上的小系统：一个服务进程 + 一个数据目录。

**不是**：

- 不是浏览器：没有 HTML/CSS/JS，没有外部链接、没有跨站跳转、没有 Cookie。
- 不是通用 Web 服务器：没有 HTTP，没有 URL 路由，没有 MIME 类型。
- 不是多用户系统：没有账号、没有登录、没有权限模型。会话只是一根随机的 16 字节编号。
- 不是匿名网络：**在应用层，客户端直连服务器，因此服务器 IP 必然暴露**（见[安全模型](#安全模型做到了什么没做什么)）。

---

## 30 秒上手

前提：Rust 工具链（1.80+）、Python 3.9+。

```bash
git clone git@github.com:Six-Fifteenths/Nac.git
cd Nac
```

**Windows（PowerShell）**

```powershell
cargo build --release
Copy-Item nac.routes.example nac.routes      # 第一次需要：站点目录
.\target\release\publish.exe --site site --data data
.\target\release\contentd.exe --data data --port 7000      # 终端 1：起服务
.\target\release\viewer.exe                                 # 终端 2：开窗口
```

**Linux / macOS**

```bash
cargo build --release
cp nac.routes.example nac.routes
./target/release/publish --site site --data data
./target/release/contentd --data data --port 7000 &
./target/release/viewer
```

窗口里应该看到一页《白塔》，地址栏是 `/1`。点链接翻页，`Esc` 返回。

> `viewer` 需要图形环境。只有命令行（比如刚装好的 VPS）时用 `probe` 验证，
> 或者在本机跑 `viewer` 通过 `nac.routes` 指向服务器。

不想开窗口、只想确认线路是否通：

```bash
./target/release/probe --addr 127.0.0.1:7000 manifest
./target/release/probe --addr 127.0.0.1:7000 page 1
./target/release/probe --addr 127.0.0.1:7000 image 1
./target/release/probe --addr 127.0.0.1:7000 submit 7 name=测试
```

---

## 写一个页面

一行都不需要 HTML。新建 `site/3.py`：

```python
# nac:title 并排排版
import nac

nac.h1("并排排版")
nac.p("下面这一整块被分到两列里。")
nac.gap()

with nac.cols(2):
    nac.h2("左")
    nac.bullets("分栏是按块切的", "不是按像素切的")
    nac.col()
    nac.h2("右")
    nac.p("这半边继续写。", nac.b("粗体"), "、", nac.i("斜体"), "、", nac.tt("等宽"))
    nac.quote("窄列里引用要短。")

nac.img("img/plan.png", 80)
nac.link(1, "回首页")
nac.done()
```

要点：

- **文件名就是页号**：`3.py` → 第 3 页，客户端地址 `/3`。
- **页名**写在第一行注释 `# nac:title ...` 里，`publish` 会读走它。
- `nac` 是 `publish` **每一代自动生成**的辅助模块，里面把本代的页号表与图片表写死，所以页面里不写 id。
- `nac.done()` 输出页面；忘了写也会在进程退出时自动输出。
- `import nac` 不需要安装：它就在同一代产物的 `scripts/` 里。

完整的函数表、标记语法、表单与会话、图片处理规则见 **[docs/authoring.md](docs/authoring.md)**。

---

## 发布

```bash
./target/release/publish --site site --data data [--archive archive]
```

做这些事：

1. 扫 `site/`，**只收 `<页号>.py`**；发现 `.page` 直接报错。
2. 每个脚本先 `ast.parse` 过一遍——语法错在发布时就停，不会变成线上空白页。
3. 图片只收 `png / jpeg / webp / gif`，**不重编码**，只按容器结构剪掉会认人的元数据段
   （EXIF / XMP / IPTC / 注释 / 时间戳），**像素逐位不变**；同时读出宽高写进 `nac.py`，
   客户端据此预留正确高度，图到了不跳。
4. 生成 `scripts/nac.py`（含本代 `PAGES` / `IMAGES` 表），写进新的代目录，最后原子更新 `data/current`。
5. 保留最近 5 代；`--archive <目录>` 时先把**原图（含 EXIF）**另存一份，发布链路只走剥干净的副本。

**服务端不需要重启**：`contentd` 每次请求都重读 `data/current`。

失败的例子（都是真实输出）：

```
publish failed: 2.py: SyntaxError: '(' was never closed
publish failed: 3.page: 不再支持 .page，改成同名 Python 页面 3.py
publish failed: img/x.bmp: 不支持 bmp；请导出成 png（无损）或 jpeg / webp / gif
publish failed: data 目录 /srv/nac/data 在 site 目录 /srv/nac 里面：上一代产物会被当成页面源文件
```

---

## 跑服务端

```bash
./target/release/contentd --data data --port 7000 --bind 127.0.0.1 \
    [--log data/access.jsonl] [--session-ttl-hours 168] [--env KEY=VALUE ...]
```

| 参数 | 默认 | 说明 |
|---|---|---|
| `--data` | `data` | 代、会话、页面私有目录都在这里 |
| `--port` | `7000` | |
| `--bind` | `127.0.0.1` | **默认只听回环**；对外暴露才改 `0.0.0.0` |
| `--log` | 关 | 给路径才写 JSONL 访问日志（不记 IP） |
| `--session-ttl-hours` | `168` | 会话文件多久算过期（后台线程每 10 分钟清一次） |
| `--env K=V` | — | 额外塞给页面脚本的环境变量（运营台用 `NAC_ROOT`） |

每个页面请求 = 启动一次 `python3 -B`（不写 `__pycache__`），**超时 2 秒、输出上限 128 KB、页面 ≤ 16 KB**。
任何失败（页不存在 / 脚本崩 / 超时 / 请求畸形）都回**逐字节相同**的 `EMPTY`，原因只进服务端 stderr 与访问日志。

访问日志长这样：

```json
{"t":1792802513,"page":1,"result":"ok:1","ms":49,"bytes":30,"sess":"37bad02f"}
{"t":1792802513,"page":99,"result":"fail","ms":0,"bytes":16,"sess":"b5d18bf9"}
```

---

## 用客户端访问

```bash
./target/release/viewer [--site 站点名] [--page 页码] [--routes 路径] [--addr 主机:端口] [--selftest]
```

**地址栏语法**（它显示什么，你就输什么）：

| 输入 | 含义 |
|---|---|
| `/7` 或 `7` | 本站第 7 页 |
| `/` | 本站首页 |
| `console/1` | 切到名为 `console` 的站点，第 1 页 |
| `nac://console/1` | 同上，带协议前缀 |
| `127.0.0.1:7000/1` | **拒绝**——端点不经过用户 |

**站点目录 `nac.routes`**：名字到端点的映射放在二进制之外，端点 base64 编码，避免文件一眼读成地址。

```
# 生成一条：viewer --routes-encode 你的主机:端口
# 检查解码：viewer --routes-check
*home   MTI3LjAuMC4xOjcwMDA=
console MTI3LjAuMC4xOjcwMDE=
```

查找顺序：`--routes <路径>` → 环境变量 `NAC_ROUTES` → **可执行文件同目录** → 当前目录 → 同名 `.example`。
带 `*` 的是默认站点。发布新内容不用动这个文件；换服务器/换端口才需要改并重新分发。

**独立分发（可选）**：编译时用 `NAC_EMBED_ROUTES` 把站点目录写进 viewer，运行时不需要 `nac.routes`：

```bash
NAC_EMBED_ROUTES='*home MTI3LjAuMC4xOjcwMDA=' cargo build --release -p viewer
# Windows 不弹控制台窗口：再加 --features windows_gui
```

`NAC_EMBED_ROUTES` 与 `nac.routes` 格式相同，token 用 `viewer --routes-encode 主机:端口` 生成；`--addr` 仍可覆盖。嵌入后 `--routes-check` 只显示站点名，不显示端点。
这只是 `obfstr` 字符串混淆，**不是保密**：进程内存、调试器和网络抓包仍能取得端点，直连也必然暴露服务器 IP。要隐藏源站需要中继或 Tor。

**快捷键**：`Ctrl+L` 定位地址栏、`Esc` / `Alt+←` 后退、`Alt+→` 前进、`F5` 或 `R` 刷新（焦点不在输入框时）、回车提交表单。

**运算符与状态**：取页在后台线程进行，跨网也不会卡住窗口；底部状态栏会显示"正在取第 N 页…"。
离开表单页再回来，已填内容会恢复；提交成功则清空。

---

## 运营台（我自己的管理界面）

同一个 `contentd`、同一个 `viewer`，只是**第二个站点**，只在本机跑：

```bash
./target/release/publish --site admin/site --data admin/data
./target/release/contentd --data admin/data --port 7001 --bind 127.0.0.1 \
    --env NAC_ROOT=$PWD --env NAC_LOG=$PWD/data/access.jsonl
```

然后客户端地址栏输 `console/1`（`nac.routes` 里要有 `console` 这一行）。

运营台的五个页面都是普通 Python 页面，直接用 `NAC_ROOT` 指到的目录读写：

| 页 | 作用 |
|---|---|
| 1 | 概览：当前代、源页面数、产物占用、会话数、appdata、日志路径 |
| 2 | 发布：在页面里跑 `publish`，显示退出码与输出 |
| 3 | 数据：留言逐行显示，按行号删除；私有目录清单 |
| 4 | 内容清单：`site/*.py` 的页号、页名、大小、改动时间 |
| 5 | 访问统计：今天/累计请求数、独立会话、结果分布、最慢一次、热门页 |

与公开站点**完全隔离**：不同 data 目录、不同会话、不同代。它只监听回环，所以没有登录——
远程访问走 `ssh -L`（见 [docs/deploy.md](docs/deploy.md)）。

> 判断依据：部署形态是单人 VPS。`127.0.0.1` 是网络位置而不是认证，能上这台机器的人本来就有
> shell 权限，再加口令等于把 SSH 做过的事再做一遍，还会把明文口令留在磁盘上。

---

## 部署到服务器

```bash
cargo build --release
sudo ./deploy/install.sh                 # 装到 /srv/nac，公开站点只监听回环
sudo ./deploy/install.sh --bind-public   # 同上，但公开站点绑 0.0.0.0（会暴露服务器 IP）
```

脚本会建 `nac` 用户、装二进制、同步源码、以 `nac` 身份发布两套站点、按目标目录改写并安装两个
systemd unit、`enable` + `restart`，最后自检（`is-active`、`ss -ltnp`、用一个裸帧取一次 manifest）。

**注意**：两个 unit 默认都只绑 `127.0.0.1`，装完外人是连不上的——这是"不暴露 IP"的当前状态。
要让普通用户访问，要先定传输层；临时直连才用 `--bind-public`。

完整流程、`ssh -L` 配方、上线清单、日志轮转、备份、升级、故障排查：**[docs/deploy.md](docs/deploy.md)**。

---

## 协议速览

```
┌──────────┬────────┬───────────────┐
│ 4B 长度  │ 1B 类型 │    payload     │
└──────────┴────────┴───────────────┘
长度 = 1 + payload.len()，≤ 32768
```

| 值 | 名称 | payload |
|---|---|---|
| `0x01` | `REQ` | `16B 会话` + `u8 op` + 参数（op 0 清单 / 1 页 / 2 图+偏移） |
| `0x02` | `SUBMIT` | `16B 会话` + `u16 页号` + `u16 字段数` + 字段 |
| `0x81` | `MANIFEST` | `16B 会话` + `u16 条数` + 条目（页号/标志/哈希/标题） |
| `0x82` | `PAGE` | `u16 页号` + 总长 + 偏移 + 本片长 + 字节 |
| `0x83` | `IMAGE` | 同上 |
| `0x84` | `EMPTY` | 16 字节 0 |

- 一连接一请求一响应，之后关闭；没有多路复用、没有魔数、没有版本字段。
- 页 ≤ 16 KB；图片按 16 KB 分片，用偏移续取。
- 所有失败都是同一个 `EMPTY`，响应字节完全相同。

细节与取舍（为什么不用 HTTP、为什么不做流量填充）：**[docs/protocol.md](docs/protocol.md)**、[PROTOCOL-DESIGN.md](PROTOCOL-DESIGN.md)。

---

## 安全模型：做到了什么，没做什么

**做到了**

- 客户端界面、错误信息、普通构建的二进制里都没有端点；端点只在 `nac.routes`，或按需在编译期通过 `NAC_EMBED_ROUTES` 嵌入（字符混淆，不是保密）。
- 地址栏不接受主机地址，普通用户没有"输入 IP"这个概念。
- 发布链路剥掉图片元数据（GPS、机型、作者、时间），**像素不变**。
- 失败统一为 `EMPTY`，不向客户端泄露失败原因；原因只在服务端日志。
- 服务端默认只监听回环；运营台不在公网可达。
- 二进制 `strip + LTO + panic=abort`，无自更新、无遥测、无版本号输出。

**没做到（重要）**

- **服务器 IP 仍会暴露给任何抓包或看文件的人**。客户端要直连就必须知道地址，
  `nac.routes` 与编译期 `NAC_EMBED_ROUTES` 只是把它变得不显眼。真正的隐藏属于传输层（Tor / 自建中继），**当前未实现**。
- 没有账号与权限：任何能连上端口的人都能读到全部页面、提交任意表单。
- 页面脚本以服务进程身份运行，拥有服务器上的读写能力。**只跑自己写的页面**；
  一旦允许别人提交 Python，那就是远程代码执行。
  为此 `contentd` 拒绝以 root 启动，unit 里也关掉了全部提权面（见 [docs/deploy.md](docs/deploy.md) §5.0）。
- 没有限流：慢页面被反复请求会堆积线程。

---

## 已知限制

- 每次翻页要起一个 Python 进程，实测约 **48 ms** 开销（本机）；要更快就得常驻进程。
- 页面 16 KB 上限，超出发布时就会被拒；没有自动分页。
- 只有 9 种排版块；没有表格、没有嵌套列表、没有视频。
- 分栏按块切，两列高度不自动对齐。
- 会话只是一根编号，没有身份；`data/state/sessions/` 里的文件按 TTL 过期删除。
- 访问日志没有内建轮转，长期跑要配 logrotate。
- Windows 上没有 rlimit，资源限制只有超时与输出上限。

---

## 仓库结构

```
crates/
  protocol/     帧结构、编解码（客户端/服务端共用）
  viewer/       egui 图形客户端：markup 解析、net、fetch（异步取页）、loader（异步取图）、routes
  contentd/     内容服务：Python 执行、会话、图片分片、访问日志
  publish/      发布工具：语法检查、图片元数据剥离、代管理
  probe/        命令行冒烟客户端
site/           示例站点（页面 + img/）
admin/site/     运营台页面
deploy/         install.sh 与两个 systemd unit
docs/           编写 / 部署 / 协议三份手册
```

---

## 文档

| 文件 | 内容 |
|---|---|
| [docs/authoring.md](docs/authoring.md) | 页面写法：`nac` 全部函数、线上标记语法、表单与会话、图片规则、发布校验 |
| [docs/deploy.md](docs/deploy.md) | 部署与运维：`install.sh`、systemd、`ssh -L`、上线清单、备份、日志、排障 |
| [docs/protocol.md](docs/protocol.md) | 协议细节：帧格式、消息语义、错误统一、会话、分片、设计取舍 |
| [PROTOCOL-DESIGN.md](PROTOCOL-DESIGN.md) | 实现规格：目录布局、渲染、界面、验收、反分析清单、保留项 |
| [PROTOCOL-DECISIONS.md](PROTOCOL-DECISIONS.md) | 早期决策记录（历史文件，保留以说明取舍过程） |

---

## 许可

**仅限非商业用途**（见 [LICENSE](LICENSE)）。允许阅读、学习、修改、自用部署；
不允许商业使用、不允许作为商业产品的一部分分发。

`gurted/`（同目录下的参考项目）不属于本仓库，仅作本地参考，已在 `.gitignore` 中排除。
