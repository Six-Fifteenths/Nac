# 实现规格

配套：[PROTOCOL-DECISIONS.md](./PROTOCOL-DECISIONS.md)

> 一个 Rust 工作区：`protocol` 帧编解码 / `viewer` 客户端 / `contentd` 服务器 / `publish` 发布工具。

---

## 0. 当前实现状态

| 部分 | 状态 |
|---|---|
| `protocol` | ✅ 编解码 + 往返/截断测试 |
| `publish` | ✅ 只收 `.py` 页面、读 `# nac:title`、语法检查、图片编号、原子切换、保留 5 代 |
| `contentd` | ✅ Python 页、会话、图片分片、统一错误（静态页读取路径保留但不再产生） |
| `probe` | ✅ 命令行冒烟客户端 |
| `viewer` | ✅ 最小浏览器：地址栏（站点名/页码）、←/→、刷新、首页、站点索引、表单、图片 |
| 寻址 | ✅ `nac.routes` 站点目录，界面不出现端点 |
| 页面形态 | ✅ 全部为 Python 脚本 + 生成的 `nac` 辅助模块 |
| 排版 | ✅ 段落/标题/列表/引用/等宽/图片/表单 + `[[cols n]]` 分栏（`[[col]]` 指定切点） |
| 取页 | ✅ 后台线程取页与提交，UI 不阻塞（跨网不再卡窗） |
| 会话清理 | ✅ `--session-ttl-hours`（默认 7 天），后台线程每 10 分钟清理 |
| 部署 | ✅ `deploy/*.service`（systemd），目录布局与备份清单见 §5.7 |
| 图片元数据剥离 | ✅ 无损段级剥离（PNG/JPEG/WebP/GIF），像素逐位不变 |
| 原图归档 | ✅ `publish --archive`：原件留本机，发布链路只有剥干净的副本 |
| 图片宽高 | ✅ 写进 manifest 与 `nac.py`，占位按真实比例，不再跳变 |
| 访问日志 | ✅ `contentd --log`，JSONL，默认关、不记 IP（§5.3） |
| 传输 | ⬜ 未定，当前只监听 `127.0.0.1:7000` |
| 资源限制 | ⬜ 仅超时与输出上限；rlimit 在 Windows 上未实现 |
| 常驻后端服务 | ⬜ 未做，已确认为将来的需要（§10.2） |
| 运营台 | ✅ `admin/` 站点 + 第二个 contentd（§5.6），`ssh -L` 远程访问 |

---

## 1. 帧协议

### 1.1 格式

```
┌──────────┬────────┬───────────────┐
│ 4B 长度  │ 1B 类型 │    payload     │
└──────────┴────────┴───────────────┘
长度 = 1 + payload.len()，≤ 32768
```

- **没有魔数、没有版本字段、没有填充**。Tor 的中继单元本来就是固定 514 字节，应用层填充不带来额外匿名性。
- 每个连接：一次请求、一次响应、关闭。没有多路复用。
- 页 ≤ 16384 字节（文本，够用）。图片按 16KB 分片，用 offset 续取。

### 1.2 类型

| 值 | 名称 | payload |
|---|---|---|
| `0x01` | `REQ` | `16B 会话` + `u8 op` + 参数 |
| `0x02` | `SUBMIT` | `16B 会话` + `u16 页号` + `u16 字段数` + N × (`u8 名长`, 名, `u32 值长`, 值) |
| `0x81` | `MANIFEST` | `16B 会话` + `u16 条数` + N × (`u16 页号`, `u8 标志`, `u64 哈希`, `u16 标题长`, 标题) |
| `0x82` | `PAGE` | `u16 页号` + `u32 总长` + `u32 偏移` + `u32 本片长` + 字节 |
| `0x83` | `IMAGE` | 同上 |
| `0x84` | `EMPTY` | 16 字节 0 |

`REQ` 的 op：`0` = 清单，`1` = 页 + `u16 页号`，`2` = 图 + `u16 图号` + `u32 偏移`。

规则：

1. 未知类型 → 回 `EMPTY`。
2. 页不存在 / 脚本崩溃 / 超时 / 口令缺省 / 请求畸形 → **同一个 `EMPTY`**，响应字节完全相同。
3. 任何解析失败 → `EMPTY`，不返回错误细节。
4. 服务器从不主动发送 `MANIFEST` 之外的任何元信息（无时间、无版本、无文件数）。
5. 会话号为全零时，服务器生成一个；只有 `MANIFEST` 会把它回传。

### 1.3 为什么不用 HTTP

HTTP 会带 `Server`、`Date`、状态行、头顺序等可指纹信息，且默认支持大量本系统不需要的方法。自定义帧只有 6 个类型，行为可穷举。

---

## 2. 页面就是 Python

`site/<页号>.py` 是唯一的内容单位——没有 `.page`，没有模板语法，页面就是一段程序。

```python
# nac:title 白塔
import nac

nac.h1("白塔")
nac.p("第 3 号信标，仍然在广播。")
nac.quote("他们把它留在这里，是因为它不会说话。")
nac.bullets("东侧门：关闭", "西侧门：有人进去过")
nac.img("img/plan.png", 80)
nac.link(2, "第二页")
nac.done()
```

- 首行注释 `# nac:title <文字>` 是这个页在清单里显示的名字，publish 读走它。
- `nac.done()` 输出页面；不写也会在进程退出时自动输出。
- 页号来自文件名；`1.py` 就是第 1 页。

### 2.1 辅助模块 nac

publish 每一代都重新生成 `scripts/nac.py`，其中把本代的页号表与图片表写死，所以页面里不用关心 id：

| 函数 | 作用 |
|---|---|
| **B 文本** `esc` `b` `i` `tt` | 转义与行内样式 |
| **C 排版** `h1/h2/h3` `p` `quote` `bullets` `code` `gap` `raw` | 块级内容 |
| **D 布局** `cols(n)` `col()` | 并排（`with` 上下文） |
| **E 媒体** `img(path, width)` | 图片 |
| **F 导航** `link` `redirect` `first_page` | 页间跳转 |
| **G 表单** `form` `field` `password` `button` | 输入与提交 |
| **H 请求** `request` `fields` `field_of` `page_id` | 本次请求 |
| **I 会话** `session` `save_session` `session_get` `session_set` | 跨请求状态 |
| **J 数据** `data_dir` `data_path` `read_json` `write_json` `read_lines` `append_line` `counter` | 站点私有目录里的文件 |
| **K 输出** `done` `flush` | 结束页面 |

### 2.2 线上标记

辅助模块最终输出的还是这套行标记，客户端只认它：

```
# 标题（也可 ## 二级）
空行分隔段落
> 引用
- 列表项
| 等宽代码行
!img 1 80% 64x32      图片（id、宽度%、可选原始宽高比）
[[2|第二页]]           页内跳转（2 为页号）
[[form 7]]            表单：提交后跳到第 7 页
[[input name|提示]]    单行输入
[[secret pass|提示]]   密码输入
[[button 提交]]        提交按钮
[[cols 2]] … [[end]]  并排布局；块按顺序分列
[[col]]               在 cols 块里强制换列
```

规则：

- 除上述前缀外，整行按普通段落处理。
- 图片号由发布工具分配（脚本里写路径，`nac.img` 查表），客户端无法用路径引用文件。
- 没有外部链接语法——协议层不存在这个字段，所以也不存在"被诱导去访问别处"。
- 页面输出若以 `@@REDIRECT n` 开头，服务器改发第 n 页。
- `nac.link()` 在运行期查页号表，线上不会出现指向不存在页的链接。
- 所有页都在发布时做语法检查；写错 Python 会在 publish 阶段直接报错，不会变成空白页。

---

## 3. 渲染（egui）

| 标记 | 控件 |
|---|---|
| `#` | `RichText::new(t).size(32.0).strong()` |
| `##` | 同上，`size(24.0)` |
| 段落 | `Label::new(t).wrap()` |
| `>` | 水平布局：3px 竖色条 + 缩进 12 + 弱化颜色 |
| `-` | 圆点 + 悬挂缩进 20 |
| `!img` | 纹理按宽度等比缩放（宽度默认 100%，不超过原始宽度） |
| `[[n\|文字]]` | 可点击 Label，悬浮变色 |

- 整页放在一个 `ScrollArea::vertical()` 里。
- 字体：优先用系统 CJK 字体（DengXian / 黑体 / 苹方 / 文泉驿），找不到再退回内置字体。
- 不在 UI 上显示端点，只显示站点名。

---

## 4. 界面

```
┌───────────────────────────────────────────────────────┐
│ ← → ⟳ ⌂  127.0.0.1:7000/2                      [索引] │
├───────────────────────────────────────────────────────┤
│                                                       │
│  # 白塔                                                │
│  第 3 号信标，仍然在广播。                               │
│                                                       │
│  ▏他们把它留在这里，是因为它不会说话。                     │
│                                                       │
│  • 东侧门：关闭                                         │
│  • 西侧门：有人进去过                                    │
│                                                       │
│  [ 去钟楼 ]                                            │
│                                                       │
├───────────────────────────────────────────────────────┤
│ ● 已连接 127.0.0.1:7000      Ctrl+L 地址 · Esc 返回 · F5 刷新 │
└───────────────────────────────────────────────────────┘
```

- 顶栏：后退、前进、刷新、首页、**地址栏**、站点索引。
- 地址栏显示的是**站点名 + 页码**，不显示、也不接受主机地址：`/2` 就是"本站第 2 页"。
- 输入接受的写法：

  | 输入 | 解释 |
  |---|---|
  | `/7` 或 `7` | 本站第 7 页 |
  | `/` | 本站首页 |
  | `<站点名>/7` | 切到另一站点并直达第 7 页 |
  | `nac://<站点名>/7` | 同上，带协议前缀 |
  | `127.0.0.1:7000/1` | **拒绝**（地址无效）——端点由站点目录决定，不经过用户 |

- 站点目录 `nac.routes`（见 §5.5）把站点名映射到端点；只有一个站点时地址栏只显示 `/页码`。
- 换站点意味着：会话号重置、图片缓存清空、历史清空——前后台不共享任何状态。
- 索引按钮列出服务器清单里的全部页（号、标题、是否动态），点击即跳转。
- 页标题只出现在窗口标题栏；页面内的 `# 标题` 仍是正文的一部分。
- 不显示任何错误细节的目的没有变，但会区分三种情况：连不上、没有这一页、提交失败（保留表单内容可重试）。**错误信息里也不出现主机地址。**
- 快捷键：`Ctrl+L` 定位地址栏、`Esc`/`Alt+←` 后退、`Alt+→` 前进、`F5` 刷新、`R` 刷新（焦点不在输入框时）。

---

## 5. 服务器与发布

### 5.1 目录

```
site/                    你写的东西
├─ 1.py                  第 1 页（Python 页面）
├─ 2.py
├─ 7.py
└─ img/plan.png

data/                    发布产物（服务器只读这里）
├─ current               当前代的目录名
├─ state/sessions/       会话文件
├─ appdata/              页面脚本的 data_dir()
└─ gen/<代>/
   ├─ manifest.json
   ├─ scripts/1.py
   ├─ scripts/2.py
   ├─ scripts/7.py
   ├─ scripts/nac.py     生成的辅助模块（含页号表/图片表）
   └─ img/1.bin
```

### 5.2 publish

```
publish --site site --data data [--archive archive]
```

1. 只收 `<页号>.py`；发现 `.page` 直接报错并提示改名。
2. `data` 不能在 `site` 里面（否则上一代产物会被当成源文件扫进来），违反直接拒绝。
   `--archive` 目录同理，不能在 `site` 里。
3. 图片只收 png/jpeg/webp/gif；bmp/tif/heic/avif 报错要求转换。
4. 图片**不重编码**，只按容器结构剪掉会认人的段，其余字节原样复制 → 像素逐位不变。

   | 格式 | 丢弃 | 保留 | 宽高来源 |
   |---|---|---|---|
   | PNG | `tEXt` `iTXt` `zTXt` `tIME` `eXIf` | IHDR/PLTE/tRNS/IDAT/IEND/gAMA/iCCP… | IHDR |
   | JPEG | `APP1`(EXIF/XMP) `APP12` `APP13`(IPTC) `COM` | APP0/APP2(ICC)/APP14 + 图像数据 | SOF |
   | WebP | `EXIF` `XMP ` 块（并清掉 VP8X 的对应标志位） | VP8/VP8L/VP8X/ALPH/ANIM/ICCP… | VP8 / VP8L / VP8X |
   | GIF | Comment Extension | 其余块（LZW 数据） | 逻辑屏幕描述符 |

   `bmp/tif/heic/avif` 直接报错要求转换；带 XMP 的 GIF 也拒绝（XMP 藏在带魔数尾的
   sub-block 里，猜错会破坏结构，宁可让人重新导出）。

   同时从容器头读出宽高，写进 manifest 与生成的 `nac.py`。
5. 每个脚本先用 `ast.parse` 过一遍，语法错在这里就停，不会变成线上空白页。
6. 从 `# nac:title` 取页名；没写就留空（客户端显示"第 N 页"）。
7. 图片按相对路径排序分配 id，写到 `img/<id>.bin`。
8. 生成 `scripts/nac.py`，把本代的 `PAGES` / `IMAGES`（含宽高）写死。
9. 写 `gen/<时间戳>/`，最后原子写 `data/current`；保留最近 5 代。
10. `--archive <目录>` 给定时，先把**原件**（含 EXIF）拷到 `archive/<相对路径>`，再剥一份发布。
    归档目录不进仓库（`.gitignore`）、不发布、永不通过协议送出——发布链路上只有剥干净的副本。
11. 不写任何日志文件。

### 5.3 contentd

```
contentd --data data --port 7000 --bind 127.0.0.1 [--log access.jsonl] [--env K=V ...]
```

- 默认只监听 `127.0.0.1`；对外暴露时才用 `--bind 0.0.0.0` 或指定网卡地址。
- 每次请求都重读 `data/current`，发布新代不需要重启。
- 每个页面请求 = 启动一次 Python（`-B`，不写 `__pycache__`），超时 2s、输出上限 128KB。
- 任何失败——页不存在、脚本崩溃、超时、请求畸形——都回同一个 `EMPTY`；原因只进 stderr。
- 不打开目录、不拼接用户输入、页号必须是 manifest 里的键。
- 单线程 accept + 每连接一个线程；短连接，处理完立即关闭。
- `--log` 给一个路径才记访问日志（JSONL，默认关、不记 IP）：

  ```json
  {"t":1792802513,"page":1,"result":"ok:1","ms":49,"bytes":30,"sess":"37bad02f"}
  {"t":1792802513,"page":99,"result":"fail","ms":0,"bytes":16,"sess":"b5d18bf9"}
  ```

  `result` 取 `manifest:N` / `ok:N`（含跳转后的最终页号）/ `image` / `image-unknown` /
  `image-missing` / `fail` / `malformed` / `no-generation`。`sess` 是会话号前 8 个十六进制字符，
  用来数"多少个人来过"，不是身份标识。响应层仍然统一 `EMPTY`，客户端看不出失败原因。
- `--env K=V` 往页面脚本里塞额外环境变量（运营台用 `NAC_ROOT`）。

### 5.4 torrc

```
HiddenServiceDir /var/lib/tor/site/
HiddenServicePort 80 127.0.0.1:7000
```

服务器防火墙不开放任何入站端口。

### 5.5 站点目录 nac.routes

客户端不认识 IP，只认识站点名。名字到端点的映射放在**二进制之外**的文本文件里：

```
# 生成一条：viewer --routes-encode 主机:端口
# 检查解码：viewer --routes-check
*home   MTI3LjAuMC4xOjcwMDA=
```

- 查找顺序：`--routes <路径>` → `NAC_ROUTES` → 可执行文件同目录 → 当前目录。
- 带 `*` 的站点是默认站点；没有 `*` 时第一条算默认。
- 端点用 base64 存，避免文件一眼读成地址。**这不是加密**：客户端要直连就必须知道端点，能动手的人总能拿到。
- 因此"用户拿不到我的 IP"在产品层面只能做到"看不到、输不进、不显示"；真正的隐藏属于传输层（Tor / 中继），见 §9。
- 发布新内容不需要动这个文件；换服务器 / 换端口才需要改，并且要重新分发客户端包。

### 5.6 运营台（我自己的管理界面）

同一个 `contentd`、同一个 `viewer`，只是第二个站点：

```
publish --site admin/site --data admin/data
contentd --data admin/data --port 7001 --bind 127.0.0.1 --env NAC_ROOT=/srv/nac
```

- 运营台自己的源文件在 `admin/site/`，产物在 `admin/data/`（不能把产物目录放进源目录：上一代会当成页面被扫进来，publish 会直接拒绝）。
- 页面：1 概览 / 2 发布（在页面里直接跑 `publish`）/ 3 数据（留言、私有目录）/ 4 内容清单。

- 与公开站点共用协议和客户端，但**数据目录、会话、代都分开**：运营台看不到访客会话，公开站也看不到运营台。
- `NAC_ROOT` 由 `--env` 传进页面脚本，脚本据此推出 `site/`、`data/` 的绝对路径。
- 发布按钮走的是真 `publish`：校验失败就不切换，旧的一代继续服务。二进制位置按 `NAC_PUBLISH` > `target/{release,debug}/publish` > `PATH` 的顺序找，部署时用 `--env NAC_PUBLISH=/srv/nac/bin/publish` 指过去最稳。
- 运营台只监听 `127.0.0.1`，不对外；远程用 `ssh -L`。

#### 远程访问（ssh -L）

服务器上（systemd / tmux 保持）：

```
contentd --data admin/data --port 7001 --bind 127.0.0.1 --env NAC_ROOT=/srv/nac
```

自己的电脑：

```
ssh -N -L 7001:127.0.0.1:7001 you@server
```

固化到 `~/.ssh/config`：

```
Host nac
  HostName server
  LocalForward 7001 127.0.0.1:7001
  ServerAliveInterval 30
```

本机 `nac.routes` 加一行（`viewer --routes-encode 127.0.0.1:7001` 生成），地址栏输 `console/1`。

为什么这条路合适：

- 服务器上不多开任何端口，7001 只在回环上；
- 认证复用 SSH 密钥，运营台不用再造一套登录；
- 流量走已有的、可审计的加密信道；
- 客户端仍然只看到 `console` 这个站点名。

代价与注意：

- 每个请求一条新连接 = 一条新 SSH 通道；图片按 16KB 分片，多图会多次往返（每次约一个 RTT）。
- 隧道断了客户端显示"无法连接"，重连即恢复（`ServerAliveInterval` 防止空闲被掐）。
- `-L` 是本地转发（你 → 服务器），不要写成 `-R`。
- 普通浏览器打不开运营台：这是自定义协议，不是 HTTP。

### 5.7 部署（Linux 服务器）

服务器上的布局：

```
/srv/nac/
├─ bin/contentd          release 构建（strip + LTO）
├─ bin/publish
├─ site/                 你写的页面（Python）
├─ admin/site/           运营台页面
├─ admin/data/           运营台的代与会话
├─ data/                 公开站点的代与数据（contentd 唯一写入的地方）
├─ access.jsonl          访问日志（--log，只有给了才写）
└─ nac.routes            可选：服务器侧自己跑客户端时才需要
```

安装（推荐用脚本，幂等，可重复运行）：

```bash
cargo build --release
sudo ./deploy/install.sh              # 装到 /srv/nac，公开站点只监听回环
sudo ./deploy/install.sh --bind-public   # 同上，但公开站点绑 0.0.0.0（会暴露服务器 IP）
```

`deploy/install.sh` 会：建 `nac` 用户 → 装二进制 → 同步 `site/`、`admin/site/` →
以 `nac` 身份发布两套站点 → 按 `--dest` 改写并安装两个 systemd unit →
`enable --now` → 自检（`is-active`、`ss -ltnp`、用裸帧取一次 manifest）。

手工等价步骤：

```bash
sudo useradd -r -s /usr/sbin/nologin nac
sudo mkdir -p /srv/nac/bin /srv/nac/admin
sudo cp target/release/{contentd,publish} /srv/nac/bin/
sudo cp -r site /srv/nac/site
sudo cp -r admin/site /srv/nac/admin/site
sudo chown -R nac:nac /srv/nac
sudo -u nac /srv/nac/bin/publish --site /srv/nac/site --data /srv/nac/data --archive /srv/nac/archive
sudo -u nac /srv/nac/bin/publish --site /srv/nac/admin/site --data /srv/nac/admin/data
sudo cp deploy/nac-{contentd,console}.service /etc/systemd/system/
sudo systemctl daemon-reload && sudo systemctl enable --now nac-contentd nac-console
```

**注意：两个 unit 默认都只监听 `127.0.0.1`。** 装完之后外人是连不上的——
这正是"看不到我 IP"的当前状态。要让普通用户访问，必须先定传输层（§9）；
临时直连才用 `--bind-public`，那时客户端能直连、服务器 IP 也就暴露了。

### 5.8 上线清单

服务器上：

1. `systemctl is-active nac-contentd nac-console` → 两个 `active`
2. `ss -ltnp | grep 700` → 只有 `127.0.0.1:7000` 与 `127.0.0.1:7001`（用了 `--bind-public` 才有 `0.0.0.0:7000`）
3. `journalctl -u nac-contentd -n 20` → 没有反复重启、没有 `bind` 失败
4. `cd /srv/nac && bin/publish --site site --data data` 手动跑一次 → `published <代> (N pages, M images)`
5. `curl` 之类打不到协议层是正常的；用 `probe` 或客户端验证
6. 访问日志：`tail -f /srv/nac/access.jsonl`，取一页应出现一行 JSON

自己电脑：

7. `ssh -N -L 7001:127.0.0.1:7001 用户@服务器`
8. `viewer --routes-check` 应显示 `home` 与 `console` 两个站点
9. 地址栏 `console/1` → 概览；`/5` → 访问统计（能看到刚产生的访问记录）
10. 在运营台点"发布" → 公开站点内容立即更新（不需要重启服务）

收尾：

11. 备份 `site/`、`admin/site/`、`data/appdata/`、`archive/`
12. `access.jsonl` 配 logrotate（没有内建轮转）
13. 防火墙只留 SSH；等传输层定了再开对应入口

运维要点：

- **重启不需要**：`publish` 之后 `contentd` 下一帧就读到新代（它每次请求重读 `data/current`）。
- **会话会自己过期**：`--session-ttl-hours`（默认 168 小时）由后台线程每 10 分钟清理一次；
  `state/tmp` 里崩溃残留的临时会话 1 小时后清掉。
- **日志会一直长**：`access.jsonl` 没有内建轮转，用 logrotate 或 `--log` 指到 journald 之外再转。
- **备份**：要备的是 `site/`、`admin/site/`、`data/appdata/`、`archive/`；
  `data/gen/` 与 `data/state/` 都是可再生的。


---

## 6. 文件布局

```
workspace/
├─ Cargo.toml            [workspace]
├─ crates/
│  ├─ protocol/          帧结构与编解码 + 标记解析（客户端/服务端共用）
│  ├─ viewer/            egui 应用、站点目录、配置
│  ├─ contentd/          服务器
│  └─ publish/           发布工具
├─ content/
└─ data/
```

`viewer` 的 release 配置：

```toml
[profile.release]
lto = true
codegen-units = 1
strip = true
panic = "abort"
```

---

## 7. 验收

| 检查 | 期望 |
|---|---|
| 帧长度 | ≤ 32768，`长度 = 1 + payload`；不接受空帧 |
| 未知类型 | 回 `EMPTY`，连接正常关闭 |
| 请求不存在的页 / 脚本出错 | 与任何其他失败的响应**字节完全相同** |
| 地址栏 | 只出现站点名与页码；输入主机地址被拒绝 |
| 客户端界面 | 任何状态、错误、提示里都不出现端点 |
| `strings viewer` | 不含端点、不含站点目录内容 |
| 图片 | 无 EXIF、无 tEXt/XMP/Comment，且像素与源文件逐位相同（png/jpeg/webp/gif） |
| 页面渲染 | 标题/段落/列表/引用/图片/跳转/表单全部正确 |
| 首屏 | 本机 < 100ms；跨网取决于传输层 |
| 断网 | 显示"无法获取内容"，不崩、不打印内部信息 |

---

## 8. 反分析检查表

- [x] release 构建：`strip`、LTO、`panic=abort`
- [x] 二进制内无端点、无口令、无密钥（端点只存在于 `nac.routes`）
- [x] 界面与错误信息不显示端点
- [ ] 不写日志、不存历史、不缓存明文到磁盘（或需显式开启并加密）
- [x] 崩溃不打印回溯
- [x] 不做自更新（少一条可被观测的网络行为）
- [x] 错误路径统一，不区分原因
- [x] 无版本号输出
- [x] 运营台只 `--bind 127.0.0.1`；`ssh -L` 不加 `-g`（§10.3）

---

## 9. 明确不做

- 搜索、书签云同步、多用户账号
- 音视频
- 外部链接（语法与协议都没有这个能力）
- 自动更新、遥测、崩溃上报
- **传输层隐藏**（Tor / 中继）：在应用层，客户端直连端点，因此"看不到我的 IP"只到"界面上没有、输不进、文件里不显眼"为止。真正的匿名性要么靠 Tor，要么靠自建中继，这属于传输决定的范畴，当前挂起。

---

## 10. 保留项与待办（已定，别忘）

### 10.1 访问日志（已完成，形态记录在此）

- 位置：`contentd`，因为它是唯一入口。形态：每请求一行 JSONL，`--log <路径>` 显式开启，默认关。
- 字段：时间、页号、结果、耗时 ms、响应字节数、会话号前 8 位。
- **不记客户端 IP**（和匿名性冲突）。若将来不走 Tor，`peer_addr` 记为保留选项，用独立开关打开，默认永远关。
- 响应层仍统一 `EMPTY`：客户端看不出失败原因，只有服务器日志分得清。
- 聚合（每页访问量、独立会话数、失败率、延迟分布）用离线脚本读日志，不在服务里做。

### 10.2 真正的后端服务（决定：将来需要一个常驻服务端）

页面脚本是"一次性进程"：没有常驻内存、没有长连接、没有后台任务。当内容长到需要这些时，加一个独立的常驻服务：

- 它持有数据库、定时任务、长连接、限流与统计；
- `contentd` 只负责把请求交给页面脚本；页面脚本再通过 localhost 或共享 SQLite 读这个服务的结果；
- 这样页面保持"薄"，超时（2s）和 128KB 上限继续有效，也不会把长任务拖进请求路径；
- 该服务与 `contentd` 分开部署、分开重启，`contentd` 挂掉不影响已积累的数据。

上线前提（缺一不可）：数据库开 WAL + 短事务；对页面脚本的并发加限流；`contentd` 的线程数上限。

### 10.3 运营台（已定：本机访问，走 nac 协议）

- 形态：第二个 `contentd` 实例当"界面服务端"，`--bind 127.0.0.1`，只给我自己用；页面见 §5.6。
- 客户端：还是 `viewer`，地址栏 `console/1`；不额外做 HTTP 后台。
- 远程：`ssh -L` 把服务器上的 7001 转到我本机，不在防火墙开任何口（recipe 见 §5.6）。
- 与公开站点完全隔离：不同 data 目录、不同会话、不同代；运营台只通过 `NAC_ROOT` 指过去读写。

**不加登录（决定，理由记录在案）**：部署形态是单人 VPS——服务器上只有我一个人有 shell，`ssh -L` 已经提供了认证与加密。回环地址不是认证而是网络位置，再加一层口令等于把 SSH 做过的事再做一遍，还会把明文口令留在磁盘上（与"不留秘密"冲突）。

因此只有两条不变量要继续守住：

1. 运营台永远 `--bind 127.0.0.1`，不绑 `0.0.0.0`，不挂反向代理。
2. `ssh -L` 不加 `-g`（`-g` 会让本机端口对局域网开放），隧道不借人。

什么时候要重新考虑认证：服务器变成多人共享、运营台要绑非回环地址、或者要把隧道给协作者。届时优先选"按需启动"（不常驻，攻击窗口为零）或改 Unix socket（把边界交回文件系统权限），而不是加明文口令。

