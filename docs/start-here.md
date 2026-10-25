# 傻瓜教程：从零到看见自己的页面

这份文档只做一件事：**手把手让你在自己电脑上写出一个能打开的页面，再把它送到服务器上。**

每一步都写了「做什么」「应该看到什么」「没看到怎么办」。命令可以直接复制粘贴。
不讲原理，原理在别的文档里（最后有一张表告诉你哪本讲什么）。

如果你只想改动线上已有的站，直接跳到 [第 11 步](#11-传到服务器)。
如果你想先搞懂"它到底在干什么"，看 [how-it-works.md](how-it-works.md)。

---

## 0. 先记住一句话

> **一个页面 = 一个 `.py` 文件。写完 → `publish` → 按 `F5`。**

`publish` 是把你的源码"印成书"，`contentd` 是把书递出去的窗口，`viewer` 是你看书的阅读器。
三样东西可以全在你自己的电脑上，也可以只在服务器上。

---

## 1. 准备（一辈子只做一次）

需要两样东西：

| 要什么 | 怎么确认 | 没有怎么办 |
|---|---|---|
| Rust | 开 PowerShell 敲 `cargo --version`，有版本号就行 | 去 <https://rustup.rs> 装 |
| Python 3 | 敲 `python --version`，要 3.9 以上 | 去 <https://python.org> 装，装的时候勾上 "Add to PATH" |

然后编译一次（**第一次要 5～6 分钟，以后改动只要几秒**）：

```powershell
cd C:\Users\SixFi\AIWork\self-protocol
cargo build --release
```

**应该看到**：最后一行是 `Finished \`release\` profile [optimized] target(s) in ...`
**没看到**：把最后 20 行红字发给我。最常见是没装 Rust，或者路径不对（`cd` 到仓库根目录，那里有 `Cargo.toml`）。

---

## 2. 搭一个试验场

不要拿真站点练手。建一个随便删的目录：

```powershell
mkdir C:\nac-lab
cd C:\nac-lab
copy C:\Users\SixFi\AIWork\self-protocol\target\release\publish.exe .
copy C:\Users\SixFi\AIWork\self-protocol\target\release\contentd.exe .
copy C:\Users\SixFi\AIWork\self-protocol\target\release\viewer.exe .
copy C:\Users\SixFi\AIWork\self-protocol\target\release\probe.exe .
mkdir mysite
```

现在的 `C:\nac-lab` 里应该有 `publish.exe`、`contentd.exe`、`viewer.exe`、`probe.exe` 和一个空的 `mysite`。

**应该看到**：`dir` 列出这五个东西，一个都不少。
**没看到**：说明第 1 步没编成功。回去看第 1 步。

---

## 3. 写第一页

用 VS Code 打开 `C:\nac-lab`，新建文件 `mysite\index.py`，把下面**整段**粘进去：

```python
# nac:title 我的第一页

import nac

nac.h1("你好，这是我写的第一页")
nac.p("这一行是正文。")
nac.gap()
nac.h2("一个小标题")
nac.bullets(
    "第一件事",
    "第二件事",
)
```

> ⚠️ **存成 UTF-8**。VS Code 右下角应该写着 `UTF-8`。写成别的编码，`publish` 会报
> `stream did not contain valid UTF-8`，中文全废。

两行要解释：

- `# nac:title 我的第一页` —— 这一页在标签页上叫这个。**必须有**，否则标签页是空的。
- `index.py` —— 这一页的地址就是 `/`（站点的首页）。别的文件名就是它们的地址。

---

## 4. 发布

```powershell
cd C:\nac-lab
.\publish.exe --site mysite --data data
```

**应该看到**：

```
published 1792940559 (1 pages, 0 images)
```

那个数字是这一版的编号（"第几代"），每次发布都不一样，正常。

**没看到**，而是 `publish failed: ...`：看 [第 12 步](#12-没反应的时候) 的排查表。

---

## 5. 开窗看

需要**两个** PowerShell 窗口。第一个窗口起服务：

```powershell
cd C:\nac-lab
.\contentd.exe --data data --port 7100 --bind 127.0.0.1
```

它会打印 `contentd listening on 127.0.0.1:7100`，然后**什么都不做了 —— 这是对的，别关它**。

第二个窗口起阅读器：

```powershell
cd C:\nac-lab
.\viewer.exe --addr 127.0.0.1:7100 --plaintext
```

**应该看到**：一个窗口，标题是「我的第一页 — nac」，地址栏写着 `dev/`，
页面正文是「你好，这是我写的第一页」。

**没看到**：

| 你看到的 | 原因 |
|---|---|
| 中间写着「无法连接到服务器」 | 第一个窗口没起，或者被关了。回去看第一个窗口 |
| 窗口根本不开 | 换了目录？必须在 `C:\nac-lab` 里敲命令 |
| 页面是空的 | 第 4 步没成功，或者 `--data data` 的路径不对 |

> `dev/` 是本地预览时临时用的名字。真站点会有真名字，比如 `nac.local/`。

---

## 6. 改一个字，看看它真的会变

在 `mysite\index.py` 里把「你好」改成「你们好」，保存。

**`contentd` 那个窗口不用碰**，就让它开着。新开一个窗口重新发布：

```powershell
cd C:\nac-lab
.\publish.exe --site mysite --data data
```

然后**在阅读器窗口里按 `F5`**。

**应该看到**：「你们好」。
**没变化**：99% 是忘了 `publish`。`contentd` 只认"印好的书"，你改了源码它不知道。

这是新手最容易卡住的地方，念三遍：**改完必须 publish。**

---

## 7. 加第二页，让它俩互相链接

新建 `mysite\contact.py`：

```python
# nac:title 联系方式
# nac:path /contact

import nac

nac.h1("联系方式")
nac.p("给我写信：someone@example.com")
nac.gap()
nac.link_path("/", "回首页")
```

再回到 `mysite\index.py`，在**最后**加一行：

```python
nac.link_path("/contact", "去联系页")
```

发布，按 `F5`：

```powershell
.\publish.exe --site mysite --data data
```

**应该看到**：首页底部多了「去联系页」，点了能跳过去。

注意 `# nac:path /contact` 这一行 —— 它让 `contact.py` 的地址变成 `/contact`。
不写的话，地址就是文件名 `/contact`，其实一样；写了可以让你**不靠文件名**决定地址。

---

## 8. 加一个表单，让它回应你

新建 `mysite\hi.py`：

```python
# nac:title 打个招呼
# nac:path /hi

import nac

name = nac.field_of("name")

nac.h1("打个招呼")
if name:
    nac.p("你好，" + name + "。")
else:
    nac.p("下面填个名字试试。")

nac.gap()
nac.form_path("/hi")
nac.field("name", "你的名字")
nac.button("说你好")
```

发布：

```powershell
.\publish.exe --site mysite --data data
```

然后在阅读器地址栏敲 `Ctrl+L`，输入 `/hi`，回车。

**应该看到**：一个输入框和一个按钮。填个字，点「说你好」，页面变成「你好，某某。」

这一步开始，页面就不是死文字了 —— 它是一个**程序**了。

---

## 9. 放一张图

1. 把图片文件直接丢进 `mysite\img\`（没有这个目录就新建）
2. 页面里加一行：`nac.figure("img/plan.png", "一张示意图", 60)`

`60` 是宽度百分比，省掉就是 100%。

发布后的输出里，`0 images` 会变成 `1 images` —— 图也进去了。

支持的格式：`png` `jpg` `jpeg` `webp` `gif`。`bmp` `heic` `avif` 会被拒绝，先转格式。

---

## 10. 想干什么，就写什么

不用背，用的时候查这张表：

| 想干什么 | 写这一行 |
|---|---|
| 大标题 / 二级 / 三级 | `nac.h1("…")` `nac.h2("…")` `nac.h3("…")` |
| 一段话 | `nac.p("…")` |
| 加粗 / 代码字体 | `nac.b("…")` `nac.tt("…")` —— 可以塞进 `nac.p` 里拼 |
| 列表 | `nac.bullets("一", "二")` / `nac.ordered("一", "二")` |
| 空一行 | `nac.gap()` |
| 一条横线 | `nac.hr()` |
| 引用 | `nac.quote("…")` |
| 表格 | `nac.table(("头1","头2"), [("a","b")])` |
| 一段代码 | `nac.code("print(1)", "python")` |
| 提示框 | `with nac.ok("标题"):` —— 还有 `nac.warn` / `nac.danger` / `nac.tip` / `nac.note` |
| 折叠块 | `with nac.details("标题"):` |
| 分栏 | `with nac.cols(2):` … `nac.col()` … |
| 图 | `nac.figure("img/x.png", "说明", 60)` |
| 链接到站内某页 | `nac.link_path("/contact", "文字")` |
| 链接到外网 | `nac.inline_url("https://…", "文字")`（放进行内文字里用） |
| 一行面包屑 | `nac.nav("/", "/notes/")` |
| 表单 | `nac.form_path("/地址")` + `nac.field` + `nac.button` |
| 读表单里的值 | `nac.field_of("字段名")` |
| 存点东西（SQLite） | `db = nac.db()`，然后 `db.execute(...)` / `db.commit()` |
| 记住上次取的会话值 | `nac.session_set("k", v)` / `nac.session_get("k")` |
| 跳转到别的页 | `nac.redirect_path("/")` |

**`print()` 不会显示在页面上** —— 它进服务端日志，用来调试。

想要完整清单和每个函数的细节：[authoring.md](authoring.md)。

---

## 11. 传到服务器

到这一步，你已经会在自己电脑上写页面了。剩下的就是"把本地的东西送上去"。

分两件事：**源码进仓库**（`git`），**页面被服务器印出来**（`publish`）。

### 11.1 本地：提交

```powershell
cd C:\Users\SixFi\AIWork\self-protocol
git add -A
git commit -m "改了首页"
git push
```

### 11.2 服务器：拉取 + 重新发布

```bash
ssh root@199.7.140.217
cd /root/Nac && git pull
sudo rm -rf /srv/nac/site && sudo cp -r site /srv/nac/site
sudo chown -R root:nac-share /srv/nac/site
sudo chmod -R u=rwX,g=rX,o=rX /srv/nac/site
sudo runuser -s /bin/sh -u nac-public -c 'umask 0007; exec /srv/nac/bin/publish --site /srv/nac/site --data /srv/nac/data --archive /srv/nac/archive'
```

**应该看到**：`published 1xxxxxxxxx (27 pages, N images)`。

**然后不用重启任何东西。** 在阅读器里按 `F5` 就是新的了。
（`contentd` 每个请求都会看一眼 `data/current` 里写的是哪一代。）

### 11.3 别的站，只换路径

| 站 | 目录 | 源码目录 |
|---|---|---|
| `nac.local` | `/srv/nac` | `site` |
| `shop.nac.local` | `/srv/nac-shop` | `site-shop` |
| `iris.out` | `/srv/nac-iris.out` | `site-iris` |
| `EX_MYTH` | `/srv/nac-EX_MYTH` | `site-myth` |

把上一条命令里的 `/srv/nac` 换成别的目录、`site` 换成对应的源码目录，就是更新那个站。

> ⚠️ **不要**拿 `deploy/add-site.sh shop` 去"更新" shop。那个脚本看到注册表里的名字是
> `shop.nac.local`、目录名是 `shop`，会以为这是个新站，给它换端口、加一条重复记录。
> 加**新**站才用它。

---

## 12. 没反应的时候

### 12.1 发布报错（`publish failed: ...`）

| 报错里写着 | 什么意思 | 怎么办 |
|---|---|---|
| `invalid character '。'` | 你把中文正文**直接写在文件里**，没放进 `nac.p("…")` 的引号 | 每一句正文都要包在 `nac.p("…")` 里 |
| `'(' was never closed` | 括号/引号少了一个 | 看它说的行号 |
| `stream did not contain valid UTF-8` | 文件不是 UTF-8 编码 | 编辑器里另存为 UTF-8 |
| `路径 /contact 被用了两次` | 两个文件要同一个地址 | 改掉其中一个的 `# nac:path` |
| `# nac:match 必须以 / 开头并以 / 结尾` | 前缀页的格式写错了 | 写成 `# nac:match /item/` |

发布失败时**旧的那一版还在正常服务**，站点不会挂。放心改。

### 12.2 窗口里报错

| 看到什么 | 什么意思 | 怎么办 |
|---|---|---|
| 「无法连接到服务器」 | 服务端没起 / 端口不对 / 关掉了 | 看那个 `contentd` 窗口还在不在 |
| 「找不到站点 xxx」 | 这个名字服务器不认识 | 检查拼写 |
| 「404 · 没有 /xxx」 | 地址没有对应的页面 | 检查有没有这个 `.py`，或 `# nac:path` 写对没 |
| 「无法获取内容」 | 页面**代码**出错了（Python 挂了） | 看第 12.3 |
| 页面没变 | 忘了 `publish`，或者忘了 `F5` | 两个都做一遍 |

### 12.3 页面代码出错，去哪看原因

窗口上只会说「无法获取内容」，**原因在服务端**。起服务时加一个日志参数：

```powershell
.\contentd.exe --data data --port 7100 --bind 127.0.0.1 --log access.jsonl
```

然后看那个窗口的屏幕：会打出完整的 Python 报错，例如

```
[page 35861] Traceback (most recent call last):
  File "...\35861.py", line 4, in <module>
    raise RuntimeError("这是我故意抛的错")
RuntimeError: 这是我故意抛的错
```

`35861` 是**页号**，不是文件名 —— 报错里的路径已经指到真正跑的那个文件了。
想知道页号对应哪个地址：

```powershell
.\probe.exe --addr 127.0.0.1:7100 manifest
```

它一行一页：页号、静态还是动态、地址、标题。

`access.jsonl` 里则是一行一条：哪个页、成功还是失败、花了几毫秒。

---

## 13. 你现在**不用**知道的事

下面这些都是真的、有用的，但**你今天不需要**。等真碰到了再回来点：

| 想知道什么 | 看哪本 |
|---|---|
| 它到底是怎么运作的（原理，不用背景知识） | [how-it-works.md](how-it-works.md) |
| 客户端每个按钮/快捷键 | [user-guide.md](user-guide.md) |
| `nac` 每个函数的完整说明、标记语法、动画 | [authoring.md](authoring.md) |
| 服务器上到底装了什么、怎么加站、怎么升级 | [deploy.md](deploy.md) |
| 证书、指纹、注册表、运营台、三种构建 | [admin-guide.md](admin-guide.md) |
| 二进制帧长什么样 | [protocol.md](protocol.md) |
| 为什么这样设计、以后能长成什么样 | [PROTOCOL-DESIGN.md](../PROTOCOL-DESIGN.md) |
| 每个取舍的代价 | [PROTOCOL-DECISIONS.md](../PROTOCOL-DECISIONS.md) |

再重复一遍最小闭环，忘了就回来看这三行：

```powershell
# 改文件 → 发布 → F5
.\publish.exe --site mysite --data data
```

---

## 附：文件都在哪

```
C:\nac-lab\                 ← 试验场，随便删
├─ publish.exe              ← 发布工具
├─ contentd.exe             ← 内容服务（发布后才能看）
├─ viewer.exe               ← 阅读器
├─ mysite\                  ← 你写的页面源码
│  ├─ index.py              → 地址 /
│  ├─ contact.py            → 地址 /contact
│  └─ img\                  ← 图片原文件丢这里
└─ data\                    ← publish 生成的，别手改
   ├─ current               ← 一个文本文件，写着"现在用第几代"
   └─ gen\1792940559\       ← 第 1792940559 代（每代都完整一份）
```

真正的仓库是同样结构，只是多几个站：`site\`（主站）、`site-shop\`、`site-iris\`、`site-myth\`。
