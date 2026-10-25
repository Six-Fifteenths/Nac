# 页面编写手册

一个页面 = 一个 Python 程序 = `site/` 下的一个 `.py` 文件。

> 第一次写页面？先看 [start-here.md](start-here.md)，那份是从零开始的一步步教程。
> 想知道为什么这样、运行的时候发生了什么？看 [how-it-works.md](how-it-works.md)。
> 这份是查手册用的。

```python
# nac:title 关于
# nac:path /about
import nac

nac.h1("关于")
nac.p("这一页的地址是 nac.local/about。")
nac.done()
```

- **路径由文件名和目录决定**：
  - `site/index.py` → `/`
  - `site/about.py` → `/about`
  - `site/notes/index.py` → `/notes/`（目录首页）
  - `site/notes/a.py` → `/notes/a`
- 想用别的路径，在文件靠前位置写 `# nac:path /custom/path`；路径必须唯一。
- **页名**来自 `# nac:title <文字>`；没有就留空，客户端显示路径。
- 内部页号仍然存在（协议要用 `u16`），但**写页面时不要依赖它**。
- `import nac` 不需要安装：每代发布都会在 `scripts/` 里生成它。
- `nac.done()` 输出页面。忘了写也没关系——进程退出时会自动输出。

---

## 0. 五分钟写第一页

```bash
# 1. 在站点目录里放一个文件
cat > site/hello.py <<'PY'
# nac:title 你好
# nac:path /hello

import nac

nac.h1("你好")
nac.p("这一页是 ", nac.tt("site/hello.py"), "，一个普通的 Python 程序。")
nac.p("现在是 ", nac.b(__import__("time").strftime("%H:%M:%S")), "。")
PY

# 2. 发布（会打印 "published <代数> (N pages, M images)"）
./target/release/publish --site site --data data

# 3. 打开（或用已经开着的客户端按 F5）
./target/release/viewer
```

地址栏写 `nac.local/hello` 就能看到。改完再跑一次 `publish` 即可，客户端刷新一次就变。

三个必须知道的点：

1. **路径由文件名决定**，`# nac:path` 只是覆盖它
2. **`import nac` 不用安装** —— 每代发布都会在 `scripts/` 里生成它
3. **`print()` 不进页面**，只进服务端日志；要显示内容用 `nac.p()`

---

## 1. 静态页、动态页、前缀页

| | 静态页 | 动态页 | 前缀页 |
|---|---|---|---|
| 标记 | 加一行 `# nac:static` | 不加（默认） | 加一行 `# nac:match /item/` |
| 何时执行 | 发布时**一次** | **每次请求** | **每次请求** |
| 回答哪个地址 | 它自己那一个 | 它自己那一个 | 以该前缀开头的**所有**地址 |
| 访问开销 | 读一个文件 | 起一次 Python（≤2 秒） | 起一次 Python（≤2 秒） |
| 能用表单/会话/数据 | **不能** | 能 | 能 |
| 适合 | 说明页、索引、目录页 | 留言板、状态、算东西 | 一条记录一页、分页、用户页 |

前三列说的是「一个页面认领几个地址」。普通页面认领一个；前缀页认领一整族：

```python
# nac:title 条目
# nac:match /hn/item/

import nac

item_id = nac.path_arg("/hn/item/")     # "/hn/item/123" -> "123"
```

于是 `/hn/item/123`、`/hn/item/abc`、`/hn/item/123/r7` 全都进这一页，页面自己
从地址里取它要的部分。浏览器那边不参与：客户端只做「最长前缀匹配」，把地址原样
交给页面。

在静态页里调用会话或数据接口会**直接报错**，并提示「去掉 `# nac:static` 即可」。
这是故意的：静态页的数据目录是渲染完就删的临时目录，写进去既不报错也不生效。

前缀页也不能是静态页：预渲染的结果只有一份，没法按地址区分。

### 1.1 支持文件：`_` 开头的都是素材

站点目录里，**任何一段路径以 `_` 开头**的东西都不是页面，而是页面的素材：

```
site/
├── _lib/
│   ├── hn.py             # 页面就 import 它
│   └── hnview.py
├── _data/
│   └── hn_seed.json      # 页面就 open() 它
└── hn/
    ├── index.py          # 这些是页面
    └── item.py
```

规则只有一条：`_lib`、`_data`、`_anything` 里的文件会**原样**复制到页面的工作
目录，页面用普通 Python 的方式拿到它们：

```python
from _lib import hn          # 目录是命名空间包，不需要 __init__.py
hn.connect()

import json, os
with open(os.path.join("_data", "hn_seed.json"), encoding="utf-8") as fh:
    seed = json.load(fh)
```

**页面的工作目录永远是 `scripts/`**，静态页和动态页一样。所以 `open("_data/x.json")`
在两种页面里都指向同一个文件。

**站点配置就放这里。** 几个页面要共用一份"名字、密码、文案、命令表"，写
`_lib/config.py`，页面 `from _lib import config` 就行 —— 不需要新机制，也不用换成
JSON：Python 模块本身就是一份能写注释、能计算的配置。`site-web.shell` 就是这么做
的：`index.py` 只管跑命令，「说什么话」全在 `_lib/shell.py` 里，换一份那个文件
就是另一个壳。

有一条边界要记住：**发布层的东西**（页号、路径、标题、`# nac:static`、`# nac:match`）
来自文件头的注释，发布时就要读，那时 Python 还没跑过 —— 所以它们没法进配置文件。

不在 `_` 目录里的非 `.py` 文件（`notes.txt`、`data.json`）也会被当作素材复制过去，
不会被静默丢掉。

### 1.2 `print()` 去哪了

`print()` 是调试工具，不该出现在页面里。所以 `nac.py` 把 `sys.stdout` 换成了
stderr：**页面打印的每一行都进服务端日志**，和 traceback 放在一起，永远到不了
读者眼前。

```python
print("starting", kind, "for", nac.path())   # 进日志
nac.p("读者看到的是这一行")                     # 进页面
```

排错时直接看 `journalctl -u nac-content -n 50`（或 contentd 的 stderr）。

## 2. nac 函数全表

### 2.0 先看分类：十个命名空间

函数一个都没少，`nac.p(...)` 照样能用。但 `nac` 底下分了十个组，
让"这一页在干什么"一眼看得出来，也让 `dir()` 和补全不再是一坨：

| 命名空间 | 管什么 | 下面几节 |
|---|---|---|
| `nac.text` | 文字与块：段落、标题、粗斜体、列表、表格、代码块 | B C |
| `nac.links` | 链接与站内导航：`link_path` / `inline_path` / `nav` / `redirect` | F |
| `nac.skin` | 面板与外观：提示框、页签、盒子、头像、图片 | D E |
| `nac.forms` | 表单：字段、按钮、预填 | G |
| `nac.req` | 这一次请求：地址参数、表单内容、是不是提交 | H |
| `nac.state` | 会话与一次性提示 | I |
| `nac.data` | 站点的数据：JSON、计数、SQLite | J |
| `nac.tty` | 终端面板（`[[term]]`） | 10 |
| `nac.site` | 本站自身：有几页、这一页叫什么 | A |
| `nac.flow` | 收尾：`done()` / `stop()` / `flush()` | K |

```python
import nac

nac.text.p("正文")
nac.text.b("重点")
nac.forms.field("title", "标题")
nac.data.db()
if nac.req.submitted():
    ...
```

两种写法等价：`nac.text.p is nac.p` 是 `True`。用哪边都行，混用也行 ——
新写的页面建议走分类，读起来更像"这一段在干什么"。

写错了会直接报出来，不会静默无效：

```python
>>> nac.text.nope
AttributeError: nac.text 里没有 'nope'
```

---

生成的 `nac.py` 分成 11 节，**只有 C/D/E/F/G 会往页面上写内容**。

### A. 环境

| 名字 | 内容 |
|---|---|
| `nac.PAGES` | 本代所有内部页号 |
| `nac.PATHS` | 路径 → 页号 |
| `nac.MATCHES` | 前缀 → 页号（前缀页认领的地址族） |
| `nac.IMAGES` | 图片表：`{"img/plan.png": (id, 宽, 高)}` |
| `nac.TITLES` | 页号 → 标题 |
| `nac.GEN` | 本代代号 |
| `nac.gen()` | 同上，函数形式 |
| `nac.pages()` | `[(页号, 路径, 标题), ...]` |
| `nac.url(page)` | 页号 → 路径 |
| `nac.title_of(page)` | 页号 → 标题 |

### B. 文本与转义

| 调用 | 说明 |
|---|---|
| `nac.esc(text)` | 转义，使其不会被当成标记（含行首的 `>` `#` `-` `+`） |
| `nac.b(text)` | 粗体 |
| `nac.i(text)` | 斜体 |
| `nac.tt(text)` | 等宽 |
| `nac.join(*parts)` | 拼成一行，`None` 自动跳过 |

### C. 块级

| 调用 | 说明 |
|---|---|
| `nac.h1/h2/h3(text)` | 三级标题 |
| `nac.p(*parts)` | 段落，参数按顺序拼接 |
| `nac.quote(*parts)` | 引用 |
| `nac.bullets(*items)` | 无序列表 |
| `nac.ordered(*items)` | 有序列表，编号自动加 |
| `nac.table(header, rows)` | 表格，第一行是表头 |
| `nac.defs(pairs)` | 键值表（两列的 `table`） |
| `nac.hr()` | 分隔线 |
| `nac.code(text, lang=None)` | 等宽块；给了 `lang` 会标出语言 |
| `nac.gap()` | 空行，分段与间距 |
| `nac.raw(line)` | 原样写一行标记，兜底用 |

```python
nac.table(
    ("名字", "说明"),
    [("甲", nac.b("第一个")), ("乙", "第二个")],
)
```

### D. 容器

| 调用 | 说明 |
|---|---|
| `with nac.cols(n=2):` + `nac.col()` | 并排成 n 列，每列宽度平分 |
| `with nac.note(kind, title):` | 提示框，`kind` 取 `info`/`ok`/`warn`/`danger` |
| `with nac.details(summary):` | 可折叠区块，默认收起 |
| `with nac.tabs(*titles):` + `nac.tab()` | 页签 |
| `with nac.box(类名, **属性):` | 样式层的盒子，见第 11 节 |
| `nac.theme(bg=…, color=…, link=…)` | 整页底色与默认字色，见第 11 节 |
| `nac.avatar(名字, 尺寸=40)` | 按名字生成的头像，见 11.5 |

容器**可以互相嵌套**，深度不限：

```python
with nac.warn("注意"):
    nac.p("提示框里可以放列表：")
    nac.bullets("甲", "乙")
    with nac.cols(2):
        nac.p("左")
        nac.col()
        nac.p("右")
```

### E. 媒体

| 调用 | 说明 |
|---|---|
| `nac.img(path, width=None)` | 插图片；`path` 是 `site/` 里的相对路径 |
| `nac.cap(text)` | 图片说明，小字灰字 |
| `nac.figure(path, caption=None, width=None)` | 图片 + 说明 |
| `nac.file(path, name=None)` | 附件：一行「文件名 · 大小 · 保存」，读者点了才取字节 |

### E2. 终端面板

| 调用 | 说明 |
|---|---|
| `nac.term(title="", target=0, bare=False)` | 开一块深色面板（`with`）；`target` 是提交目标页号 |
| `nac.term_ascii(lines)` | 横幅美术字；给一行或一串行 |
| `nac.term_line(*pairs)` | 一行彩色文字，角色与文字交替给 |
| `nac.term_blank()` | 空一行 |
| `nac.term_cursor(name, *pairs)` | 彩色前缀 + 可见输入框 |
| `nac.term_passwd(name, *pairs)` | 同上，输入显示成点 |

角色名用 `user` / `host` / `prompt` / `out` / `cmd` / `link` / `keys` / `in`，
也可以把 `(角色, 文字)` 元组直接塞进去。详见 §10。

### F. 导航

| 调用 | 说明 |
|---|---|
| `nac.link_path(path, text)` | 跳到路径；目录要带尾斜杠（如 `/notes/`）。前缀页下的地址也认 |
| `nac.link(page, text)` | 按内部页号跳转；页号不存在时整行不显示 |
| `nac.nav(*paths)` | 一行面包屑，每个路径都是行内链接 |
| `nac.home()` | 回首页那一行 |
| `nac.redirect_path(path)` | 按路径立刻跳转；路径不存在时不动 |
| `nac.redirect(page)` | 按内部页号跳转，本页已拼的内容丢弃 |
| `nac.first_page()` | 本代最小内部页号 |
| `nac.inline_path(path, text)` | 返回一个行内路径链接，塞进 `p()` 里用 |
| `nac.inline_url(url, text)` | 返回一个站外链接，点开交给系统浏览器 |

**段落里的行内链接**不会独占一行：

```python
nac.p("细节见 ", nac.inline_path("/notes/", "笔记目录"), " 那一页。")
```

站外链接只接受 `http` / `https`；别的协议会被当成普通文字，页面上连链接都不是。
点击站外链接不会改变客户端里的地址栏 —— 它打开的是系统浏览器。

`redirect_path` 会把**地址**一起带过去：跳到 `/hn/item/42` 时那一页从 `nac.path()`
里仍然看得见 `/hn/item/42`，不会退化成它自己的 `/hn/item`。

### G. 表单

| 调用 | 说明 |
|---|---|
| `nac.form_path(path)` | 声明表单提交到某个路径 |
| `nac.form(page)` | 声明表单提交给内部页号 |
| `nac.field(name, label="", secret=False)` | 单行输入 |
| `nac.password(name, label="")` | 密码输入 |
| `nac.textarea(name, label="")` | 多行输入 |
| `nac.select(name, label, options)` | 下拉选择，`options` 是字符串列表 |
| `nac.check(name, label="")` | 复选框；勾上提交 `"1"`，否则空串 |
| `nac.prefill(name, value)` | 给字段填上初始文字；只在读者还没动过它时生效 |
| `nac.hidden(name, value)` | 看不见的字段，跟着表单一起提交（记住「刚才是哪一条」） |
| `nac.button(label)` | 提交按钮 |

一页里只让有**一个**表单：表单目标记录在页面状态里，后面的声明会盖掉前面的。
需要「每行一个按钮」时，把动作做成路径（`/hn/item/12/r7`）而不是表单。

`hidden()` 是表单的记事本 —— 提交按钮不会告诉你自己属于哪一行，隐藏字段会：

```python
nac.form_path("/hn/vote")
nac.hidden("item", str(row["id"]))
nac.hidden("back", nac.path())     # 投完票回到这里
nac.button("▲ 投一票")
```

### H. 请求 · 仅动态

| 调用 | 说明 |
|---|---|
| `nac.request()` | `{"page": 页号, "path": 地址, "fields": {...}}` |
| `nac.path()` | 本次请求的完整地址，如 `/hn/item/123` |
| `nac.path_arg(prefix=None)` | 地址里前缀之后的那一段，去掉首尾斜杠 |
| `nac.fields()` | 只要字段字典 |
| `nac.field_of(name, default="")` | 取一个字段并去掉首尾空白 |
| `nac.submitted()` | 这次请求带表单内容吗 |
| `nac.checked(name)` | 复选框是否勾上 |
| `nac.page_id()` | 当前页号 |
| `nac.flash(message)` | 留言给下一个页面看（配 `redirect_path`） |
| `nac.flashes()` | 取出并清空待显示的留言 |

`path()` 和 `path_arg()` 是前缀页的入口：

```python
arg = nac.path_arg("/hn/item/")     # "/hn/item/12/r7" -> "12/r7"
head, _, rest = arg.partition("/")  # "12", "r7"
```

```python
if nac.submitted():
    nac.flash("已保存")
    nac.redirect_path("/list")
for line in nac.flashes():
    nac.p(nac.b(line))
```

### I. 会话 · 仅动态

| 调用 | 说明 |
|---|---|
| `nac.session()` | 会话状态字典（跨页、跨请求） |
| `nac.save_session(state)` | 写回状态 |
| `nac.session_get(key, default=None)` | 读一个键 |
| `nac.session_set(key, value)` | 写一个键 |
| `nac.session_update(**values)` | 一次写多个键，只读写文件一次 |
| `nac.session_pop(key, default=None)` | 取走并删除 |

会话存在服务端，按客户端带来的会话号区分；关掉客户端再回来仍然在（默认保留 168 小时）。

### J. 站点数据 · 仅动态

| 调用 | 说明 |
|---|---|
| `nac.data_dir()` | 私有目录路径（自动创建） |
| `nac.data_path(name)` | 私有目录里的某个文件 |
| `nac.read_json(name, default=None)` | 读 JSON；不存在或坏了返回默认值 |
| `nac.write_json(name, value)` | **原子**写 JSON |
| `nac.update_json(name, fn, default=None)` | **带锁**的读-改-写 |
| `nac.read_lines(name)` | 读文本行；不存在返回 `[]` |
| `nac.append_line(name, text)` | 追加一行 |
| `nac.counter(name, step=1)` | 原子地加一并返回新值 |
| `nac.db(name="site.db")` | SQLite 连接（WAL，`row_factory=Row`） |

需要一个真正的数据库时用 `db()`——标准库 `sqlite3`，不联网、不需要服务：

```python
db = nac.db()
db.execute("create table if not exists msg(id integer primary key, body text, at text)")
if nac.submitted():
    db.execute("insert into msg(body, at) values(?, ?)", (body, now))
    db.commit()
rows = db.execute("select body, at from msg order by id desc limit 10").fetchall()
```

参数化 SQL 意味着留言内容不会被当成 SQL 执行；文本永远只是文本。

### K. 输出

| 调用 | 说明 |
|---|---|
| `nac.done()` / `nac.flush()` | 输出页面（不调用也会在退出时自动输出） |
| `nac.stop()` | 这一页到此为止：已写的内容发出去，后面不再执行 |

页面是平铺的脚本，没有函数可以 `return`。提前收尾用 `stop()`：

```python
if story is None:
    nac.h2("没有这一条")
    nac.stop()          # 后面拼评论树的代码不会跑
```

所有写文件的地方都是**原子写**；`field/button/link/table` 会自动清掉分隔符（`|`、`]]`），
所以你不用手动防排版破坏。

## 3. 完整例子：留言板

```python
# nac:title 留言板
# nac:path /guestbook

import time

import nac

db = nac.db()
db.execute(
    "create table if not exists msg("
    "id integer primary key, name text, body text, at text)"
)

name = nac.field_of("name")[:40]
body = nac.field_of("message")[:200]

if body and nac.checked("public"):
    db.execute(
        "insert into msg(name, body, at) values(?, ?, ?)",
        (name or "匿名", body, time.strftime("%Y-%m-%d %H:%M")),
    )
    db.commit()
    nac.session_set("name", name or "匿名")
    nac.flash("留言已记下")
    nac.redirect_path("/guestbook")

rows = db.execute("select name, body, at from msg order by id desc limit 12").fetchall()

nac.h1("留言板")
for line in nac.flashes():
    with nac.ok("刚完成"):
        nac.p(line)

if rows:
    nac.table(
        ("名字", "留言", "时间"),
        [(r["name"], r["body"], nac.tt(r["at"])) for r in rows],
    )
else:
    nac.p("还没有人留言。")

nac.form_path("/guestbook")
nac.field("name", "你的名字（可留空）")
nac.textarea("message", "留言，最多 200 字")
nac.check("public", "我知道留言是公开的")
nac.button("留下")
nac.home()
nac.done()
```

要点：**页面自己负责一切**——读数据、写数据、判断是否需要跳转。没有框架，也没有中间层。

## 4. 线上标记

> 想知道"全部能画什么、画成什么样、比 HTML 少了什么"，看
> [rendering.md](rendering.md)。这一节讲的是写法，那一页讲的是能力边界。



客户端只认这套行标记；`nac` 是生成它们的便捷方式，`nac.raw()` 可以直接写。
**块容器可以嵌套**，只要 `[[end]]` 对得上。

```
# 标题          ## 二级        ### 三级
普通文字行       （连续的行合成一个段落）
> 引用          （连续的 > 行合成一个引用块）
- 列表项        （连续的 - 行合成一个列表）
1. 有序项       （连续的 数字. 行合成有序列表，编号由客户端重排）
| 等宽行        （连续的 | 行合成代码块）
---             分隔线
空行            分段

!img 1 80% 640x360     图片：id、宽度%、可选原始宽高比
!cap 图片说明           小字灰字，通常紧跟在图后面

[[2|第二页]]            整行跳转（页号）
[[path:/notes/|笔记]]   整行跳转（路径）
[[url:https://x/|外链]] 整行跳转（系统浏览器）
段落里写 [[path:/x|这样的]] 就是行内链接，不独占一行。

[[form 7]]              表单提交目标
[[input name|提示]]      单行输入
[[secret pass|提示]]     密码输入
[[textarea body|提示]]   多行输入
[[select mode|提示|甲;乙;丙]]   下拉选择
[[check agree|同意]]     复选框
[[hidden item|42]]      看不到的字段，跟着表单提交
[[prefill name|原文]]    给字段一个初始值（只在读者没动过时生效）
[[button 提交]]          按钮

[[cols 2]] … [[col]] … [[end]]          并排布局
[[table]] | 表头 | … | 表体 | [[end]]    表格，第一行是表头
[[note warn|标题]] … [[end]]            提示框，kind 取 info/ok/warn/danger
[[details 标题]] … [[end]]              可折叠区块
[[tabs 甲;乙]] … [[tab]] … [[end]]      页签
[[code python]] … [[end]]               带语言名的代码块
[[scene 640x200 3.5]] … [[end]]         会动的画面，见第 9 节
[[term panel|标题|7]] … [[end]]         终端面板，见第 10 节
```

行内：`*粗*`、`_斜_`、`` `等宽` ``，反斜杠 `\` 转义；`\n` 在文本里会被压成空格。

**未知指令**（`[[nonsense]]`）不会被吞掉，会当成普通文字显示——这样版本不一致时你能立刻看见。

## 5. 分栏

```python
with nac.cols(2):
    nac.h2("左")
    nac.bullets("第一项", "第二项")
    nac.col()                      # 换到下一列
    nac.h2("右")
    nac.p("右边的内容。")
```

- 不写 `nac.col()` 时按**块数**平均分配到 n 列
- 写了 `nac.col()`，切点完全由你决定，`n` 被忽略（列数 = 切点 + 1）
- 最多 4 列；列宽由客户端按可用宽度平分，两列高度不会自动对齐
- 可以嵌套，里面还能放表格、提示框、页签

## 6. 图片与附件

1. 把图片放进 `site/`（例如 `site/img/plan.png`），**直接放原始文件**
2. 页面里 `nac.figure("img/plan.png", "一张图", 80)` 或 `nac.img("img/plan.png", 80)`
   （`80` 是宽度百分比，省略即 100%）
3. 发布时：
   - 支持 `png` / `jpg` / `jpeg` / `webp` / `gif`
   - **不重编码**，只按容器结构剪掉元数据段，像素逐位不变：

     | 格式 | 丢掉 | 保留 |
     |---|---|---|
     | PNG | `tEXt` `iTXt` `zTXt` `tIME` `eXIf` | 其余全部 |
     | JPEG | `APP1`(EXIF/XMP) `APP12` `APP13`(IPTC) `COM` | APP0/APP2(ICC)/APP14 + 图像数据 |
     | WebP | `EXIF` `XMP ` 块（并清 VP8X 标志位） | 其余块 |
     | GIF | Comment Extension | 其余块 |

   - `bmp / tif / heic / avif` 会被拒绝，要求先转换；带 XMP 的 GIF 也会被拒绝
   - 宽高会写进 `nac.py`，客户端据此在图片到达前预留正确高度
4. 想保留带元数据的原图：`publish --archive <目录>`。归档目录不能放在 `site/` 里。

已知取舍：EXIF 的 `Orientation` 会一起去掉，所以竖拍照片的旋转问题不因剥离而改变。

### 附件（`_files/`）

1. 把文件放进 `site/_files/`，**平铺，不要子目录**，每个 ≤ 2 MB
2. 页面里 `nac.file("钥匙.zip")`（写 `_files/` 之后的那一段）
3. 页面上出现一行「文件名 · 大小 · 保存」。读者点了客户端才去取字节，
   取到直接写进系统下载目录：
   - 文件名由客户端净化（路径分隔符、控制字符、`..`、`CON` 这类都削掉）
   - 重名不覆盖，自动变成 `钥匙 (2).zip`；可以反复点
   - 页面说不出路径，只能给名字
4. 附件原件同时会复制进 `scripts/`，所以页面自己也能 `open("_files/钥匙.txt")` 读
5. 发布时不修改内容，原样发送

它和图片共用同一条二进制通道（`IMAGE` 帧，分块），所以协议没有为它加新东西。

## 7. 发布时的校验

| 情况 | 结果 |
|---|---|
| 两个页面路径相同 | 报错，指出冲突的路径 |
| `# nac:path` 为空或含 `..` / `?` / 竖线 | 报错，指出文件与原因 |
| `# nac:match` 不以 `/` 开头并以 `/` 结尾 | 报错 |
| 静态页声明 `# nac:match` | 报错（预渲染只有一份，没法按地址区分） |
| Python 语法错 | 报错，带文件名与原因 |
| 文件带 BOM | 自动摘掉（Notepad 存的常见） |
| 静态页里用了会话/站点数据 | 报错，说明去掉 `# nac:static` 即可 |
| 图片扩展名不支持 | 报错，指出该转成什么 |
| `data` 或 `archive` 放在 `site` 里 | 报错（会把上一代产物当素材） |
| 静态页输出超过 16 KB | 报错 |

`_` 开头的目录**不参与**页面校验：它们不是页面，语法/路径都不查（所以素材里放
一个只给别的进程用的脚本也不会报错）。

发布失败时**旧的一代继续服务**：新代写完才原子更新 `data/current`。

## 8. 限制

| 项 | 上限 |
|---|---|
| 页面正文 | 16 KB |
| 页面 stdout | 128 KB（超出视为失败） |
| 执行时间 | 2 秒 |
| 页面 stderr | 只采集 8 KB 进日志 |

任何失败都回空响应，客户端只看到「无法获取内容」。原因去
`journalctl -u nac-contentd` 找。

## 9. 会动的画面：`[[scene]]`

文字排得再好也不动。要动就给客户端一条**时间轴**：它不跑脚本，只按时间在关键帧
之间插值，把矩形画出来。所以动画是 60 帧的，而服务端只被请求了一次。

```
[[scene 宽x高 秒数]]
[[bar 颜色 宽 高]]      一块矩形：颜色 rrggbb、宽、高
[[at 秒 x y]]           它的关键帧：那一刻左上角在哪
[[at ...]]
[[bar ...]]             下一块
[[end]]
```

- 坐标是**场景自己的逻辑坐标**，客户端等比缩放到窗口宽度（最多放大 1.6 倍）
- 关键帧按时间排序后生效；第一个之前保持第一个位置，最后一个之后保持最后一个位置
- 段与段之间用 smoothstep 插值 —— 起手和收手都是缓的，不是匀速直线
- 块按声明顺序画，后声明的压在上面（所以柱子先写，盘子后写）
- `秒数` 是总时长；到点就停。**点画面从头播**

汉诺塔那一页就是这么写的。页面只做算术：

```python
lines = ["[[scene 640x142 8.70]]"]
for peg in range(3):
    lines.append("[[bar 6b7280 8 72]]")          # 柱子
    lines.append("[[at 0 %d 30]]" % (110 + 210 * peg - 4))
lines.append("[[bar c0392b 176 16]]")            # 最大的盘子
lines.append("[[at 0.00 22 84]]")                # 起点
lines.append("[[at 2.78 22 6]]")                 # 抬起
lines.append("[[at 3.22 442 6]]")                # 平移
lines.append("[[at 3.50 442 84]]")               # 落下
lines.append("[[end]]")
for line in lines:
    nac.raw(line)
```

一个 `[[bar]]` 只写一次，关键帧都挂在它名下。**写成两块**会让后一块从第 0 秒起就
画在终点上 —— 同一条指令写两次等于画了两条。

因为一切都是数字，这套东西不止能画汉诺塔：进度条、柱状图、小球、图示流程都行。
`site/_lib/hanoi.py` 是完整可抄的例子。

**体积**：一行 `[[at 8.70 442 6]]` 约 20 字节。7 层汉诺塔 127 步约 10 KB，
低于页面 16 KB 的上限；再多就该减少关键帧，或者分几页。

## 10. 终端面板：`[[term]]`

想让一页看起来像终端，`[[term]]` 画一块深色面板，里面的文字按角色上色，
最后一行可以带一个输入框 —— **输入框在框里面**，光标和提示符不会分家。

```
[[term panel|标题|表单目标页号]]
[[ascii 美术字]]                     等宽、固定行距、不折行
[[line 角色|文字|角色|文字]]          一行彩色文字
[[cursor 字段名|角色|文字]]           这一行末尾接一个输入框
[[passwd 字段名|角色|文字]]           同上，但输入显示成点
[[end]]
```

角色就这几个：`pu` 用户名、`ph` 主机名、`pd` 提示符、`out` 普通输出、
`cmd` 命令名、`link` 链接、`keys` 按键提示、`in` 读者敲的内容。

`|` 是分隔符，文字里的 `|` 会被换成全角。`panel` 是带标题栏的版式；
换成 `bare` 就去掉标题栏、整块变成纯黑底白字（WebShell 那个彩蛋的样子）。

表单目标页号给了之后，**在输入框里按回车就会提交到那一页**。所以一块终端
自带表单，不需要在外面再套一个 `nac.form_path()`。

写页面时不用手拼这些，`nac` 里有对应的函数：

```python
with nac.term("WebShell.x64_x86", target=nac.page_id()):
    nac.term_ascii(ART)                       # 一行或一串行
    nac.term_line("cmd", "'help'", "out", "  列出所有命令")
    nac.term_cursor("cmd", "pu", "visitor", "pd", ":$ ~ ")
```

`site-web.shell/index.py` 是完整例子：滚动历史、命令分发、密码门、彩蛋都在里面。

> 终端面板是**一块画出来的东西**，不是焦点：它不接管键盘，也没有滚动条。
> Tab 补全、上下翻历史这类客户端才做得到的事，这里没有。

## 11. 样式层：`[[sheet]]` 与 `[[box]]`

这一节讲的是「客户端里的那个小样式层」。它不是 CSS，而且**刻意不是**：

> 盒子只描述「这一块**是什么**」，不描述「它**多大、在哪**」。
> 宽度、位置、什么时候变，都由客户端决定。

所以这一层里没有 `width`、没有 `position`、没有百分比、没有媒体查询 ——
同一个页面在 700px 的窗口和 2000px 的窗口里都得站得住。

```
[[sheet]]
.card { bg: #ffffff; border: 1px:#dddddd; radius: 4; shadow: 8,2:#0000001f; }
.nav  { bg: #5cb85c; color: #ffffff; weight: bold; link: #ffffff; }
[[end]]

[[box class=card]]
  盒子里的东西
[[end]]
```

`[[sheet]]` 是整页的样式表，一行一条规则，选择器只能是**一个类名**（`.card` 或 `card` 都行）；
同名后者覆盖前者。`.a .b`、`.a > b`、`a:hover`、`[x]`、`@media` 这些写法会被**直接丢掉**，
不会悄悄生效。

`[[box]]` 是一个盒子，属性写在开标签里，也可以只用 `class` 让样式表来定。

### 11.1 全部属性，只有这些

| 属性 | 值 | 说明 |
|---|---|---|
| `class` | 名字 | 从样式表取一条规则 |
| `bg` / `color` | `#rrggbb`、`#rgb`、`#rrggbbaa`、`white`、`black` | 底色 / 文字色（继承） |
| `border` | `1:#dddddd` | 线宽:颜色 |
| `radius` | `4` | 圆角 |
| `shadow` | `8` 或 `8,2` 或 `8,2:#0000001f` | 模糊半径[,向下偏移][:颜色] |
| `opacity` | `0.6` 或 `60` | 整块透明度，两种写法都是 60% |
| `ta` | `center` / `left` / `right` | 盒子里**文字**的对齐 |
| `size` / `weight` | `16` / `bold` | 字号、字重（继承） |
| `font` | `sans` / `serif` / `mono` | 字体族（继承，见 11.4） |
| `italic` | `1` | 斜体（继承） |
| `link` | `#5cb85c` | 盒子内链接的颜色（继承） |
| `nowrap` | `1` | 不折行（导航、标签行） |

数值只接受 `16` 或 `16px`。`50%`、`calc(...)`、`2em` 会被**当成没写**，因为它不是数字。

**没有内边距、没有间距、没有宽度、没有方向。** 带底色或描边的盒子由客户端统一给
14×12 的内边距；再想要别的间距，不是页面能说的事。

```python
nac.sheet(".card { bg: #fff; border: 1px:#ddd; radius: 4; shadow: 8,2:#0000001f; }")

with nac.box("card"):                 # 位置参数是类名
    nac.h3("标题")
    nac.p("正文")

with nac.cols(2):                     # 并排成两栏，宽度客户端平分
    nac.p("左边")
    nac.col()
    nac.p("右边")

nac.avatar("张三", 28)                # 见 11.4
```

`class` 是 Python 关键字，所以类名用第一个位置参数传，或者写 `cls="card"`。

### 11.1.1 整页底色：`nac.theme()`

盒子管自己那一块。整页的底子是另一件事，用 `nac.theme()`：

```python
nac.theme(bg="#0d0d0d", color="#e8dfc8", link="#8a0303")
```

它只收三个颜色：`bg` 内容区底色、`color` 默认字色、`link` 默认链接色。
写一次就够，客户端铺在正文底下 —— 所以一张浅色羊皮卷可以坐在近黑的夜里。

它**只管颜色**：改不了标签栏和地址栏（那是窗口的），也改不了 860 的宽度和留白
（那是版式）。

### 11.2 没有横排

盒子只能上下堆叠。想并排，唯一的办法是 `nac.cols(n)`：它说的是"这几段并排放"，
每列多宽由客户端平分，页面说不上话。盒子里不能塞横排。

这不是没做完，是决定：一旦页面能指定"两栏按 2:1 分"，它就开始承担布局引擎的
工作，而那是另一个项目。

### 11.3 这一层不会长成 CSS

判据只有一句：**属性只能描述「这一块是什么」，不能描述「它在哪、多大、什么时候变」。**

以下这些永远不会有，问「能不能加」的时候先看这张表：

`position` `float` `z-index` `overflow` `display:grid/table` `flex-wrap`
`justify-content` `宽度/百分比` `calc()` `选择器（后代/相邻/属性/伪类）`
`::before` `::after` `content` `@media` `@container` `@import` `@keyframes`
`CSS 变量` `!important` `url()` `@font-face` `transform` `box-sizing`

理由不是洁癖：这些东西一旦进来，客户端就得实现一个布局求解器和一个层叠引擎，
而那是另一个项目（也是全部 CSS 玄学的来源）。

### 11.4 字体：三种，都编在客户端里

| `font=` | 用的字 | 用在哪 |
|---|---|---|
| `sans`（默认） | Titillium Web + CJK 兜底 | 导航、按钮、小字 |
| `serif` | PT Serif + CJK 兜底 | 正文、大标题 |
| `mono` | 系统等宽 + CJK 兜底 | 代码 |

字体文件跟着客户端一起编译进二进制，许可证是 SIL OFL
（见 `crates/viewer/assets/fonts/OFL-*.txt`）。所以同一个页面在谁的电脑上都是同一副字。

中文不在这两个字体里，会落到系统 CJK 字体（Windows 上是等线）。
这是故意的：CJK 字体动辄十几兆，编进客户端不值得。

### 11.5 头像：`[[avatar 名字 尺寸]]`

```python
nac.avatar("张三")        # 40
nac.avatar("张三", 28)    # 卡片里的小头像
```

名字决定颜色，名字的第一个字母印在上面。**不走网络、不读文件**：
同一个名字永远长同一张脸，页面也没法借头像让客户端去连别的机器。

想放一张真照片，用 `nac.img()`（图片在发布时就打包好了，见第 6 节）。

### 11.6 写法上的三个坑

- 连续两行非空文字属于同一段，盒子不会替你分段 —— 想分开就 `nac.gap()`
- `nac.sheet()` 逐行发出去，所以**一条规则必须写在一行里**；跨行会被拆坏
- 一张样式表写在页面开头；写在后面也认，因为渲染前会先扫一遍

对照用的文件在 `deploy/`：`reference.html` 是同一份样式表的手写 CSS 版，
`ours.png` + `compare.html` 是客户端和浏览器并排的结果。

## 12. 页面出错时怎么查

页面出问题**不会**在客户端显示原因（一律回空响应），因为原因可能包含服务器内部信息。
它只写服务端日志：

```bash
journalctl -u nac-contentd -n 50        # 线上
# 本地起 contentd 的话，看它的 stderr
```

日志里会看到类似 `[page 65206] Traceback (most recent call last): ...`。

| 症状 | 多半是 |
|---|---|
| 客户说「页面空白」，日志里有 `Traceback` | 页面抛异常了，看最后一行的异常类型 |
| 日志里 `timeout or wait failure` | 超过 2 秒。把计算挪到发布时（`# nac:static`），或减少工作量 |
| 日志里 `SyntaxError` | **不该出现**：发布时用 `compile()` 检查过。出现说明是手工改了 `data/gen/` |
| 页面 404，但文件在 | 路径被 `# nac:path` 改了；或目录首页不叫 `index.py` |
| 前缀页全 404 | `# nac:match` 必须以 `/` 开头**并以 `/` 结尾** |
| `ModuleNotFoundError` | 模块不在 `_lib/`（或其它 `_` 开头的目录）里；只有那里的文件会被带过去 |
| 页面内容比预期少 | 超过 16 KB 会被判失败（整页回空），不是截断 |
| `print()` 的东西不见了 | 正常：它进日志，不进页面 |
| 表单提交后行为不对 | 前缀页的表单要用 `nac.hidden()` 带上「是哪一条」，见 §G |

本地快速复现某一页（跳过协议，直接跑脚本）：

```bash
cd data/gen/<代数>/scripts
NAC_INPUT='{"page":1,"path":"/hello","fields":{}}' \
NAC_PATH=/hello NAC_DATA_DIR=/tmp/appdata \
python -B 12345.py            # 文件名就是页号
```

stdout 就是页面内容，stderr 就是日志。