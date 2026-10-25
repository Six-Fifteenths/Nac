# 页面编写手册

一个页面 = 一个 Python 程序 = `site/` 下的一个 `.py` 文件。

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

## 1. 静态页与动态页

| | 静态页 | 动态页 |
|---|---|---|
| 标记 | 加一行 `# nac:static` | 不加（默认） |
| 何时执行 | 发布时**一次** | **每次请求** |
| 访问开销 | 读一个文件 | 起一次 Python（≤2 秒） |
| 能用表单/会话/数据 | **不能** | 能 |
| 适合 | 说明页、索引、目录页 | 留言板、状态、算东西 |

在静态页里调用会话或数据接口会**直接报错**，并提示「去掉 `# nac:static` 即可」。
这是故意的：静态页的数据目录是渲染完就删的临时目录，写进去既不报错也不生效。

## 2. nac 函数全表

生成的 `nac.py` 分成 11 节，**只有 C/D/E/F/G 会往页面上写内容**。

### A. 环境

| 名字 | 内容 |
|---|---|
| `nac.PAGES` | 本代所有内部页号 |
| `nac.PATHS` | 路径 → 页号 |
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
| `nac.esc(text)` | 转义用户输入，使其不会被当成标记 |
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
| `with nac.cols(n=2):` + `nac.col()` | 并排成 n 列（最多 4） |
| `with nac.note(kind, title):` | 提示框，`kind` 取 `info`/`ok`/`warn`/`danger` |
| `with nac.tip(t)` / `ok` / `warn` / `danger` | 上面那个的简写 |
| `with nac.details(summary):` | 可折叠区块，默认收起 |
| `with nac.tabs(*titles):` + `nac.tab()` | 页签 |

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

### F. 导航

| 调用 | 说明 |
|---|---|
| `nac.link_path(path, text)` | 跳到路径；目录要带尾斜杠（如 `/notes/`） |
| `nac.link(page, text)` | 按内部页号跳转；页号不存在时整行不显示 |
| `nac.nav(*paths)` | 一行面包屑，每个路径都是行内链接 |
| `nac.home()` | 回首页那一行 |
| `nac.redirect_path(path)` | 按路径立刻跳转；路径不存在时不动 |
| `nac.redirect(page)` | 按内部页号跳转，本页已拼的内容丢弃 |
| `nac.first_page()` | 本代最小内部页号 |

**段落里的行内链接**不会独占一行：

```python
nac.p("细节见 ", "[[path:/notes/|笔记目录]]", " 那一页。")
```

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
| `nac.button(label)` | 提交按钮 |

### H. 请求 · 仅动态

| 调用 | 说明 |
|---|---|
| `nac.request()` | `{"page": 页号, "fields": {...}}` |
| `nac.fields()` | 只要字段字典 |
| `nac.field_of(name, default="")` | 取一个字段并去掉首尾空白 |
| `nac.submitted()` | 这次请求带表单内容吗 |
| `nac.checked(name)` | 复选框是否勾上 |
| `nac.page_id()` | 当前页号 |
| `nac.flash(message)` | 留言给下一个页面看（配 `redirect_path`） |
| `nac.flashes()` | 取出并清空待显示的留言 |

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
段落里写 [[path:/x|这样的]] 就是行内链接，不独占一行。

[[form 7]]              表单提交目标
[[input name|提示]]      单行输入
[[secret pass|提示]]     密码输入
[[textarea body|提示]]   多行输入
[[select mode|提示|甲;乙;丙]]   下拉选择
[[check agree|同意]]     复选框
[[button 提交]]          按钮

[[cols 2]] … [[col]] … [[end]]          并排布局
[[table]] | 表头 | … | 表体 | [[end]]    表格，第一行是表头
[[note warn|标题]] … [[end]]            提示框，kind 取 info/ok/warn/danger
[[details 标题]] … [[end]]              可折叠区块
[[tabs 甲;乙]] … [[tab]] … [[end]]      页签
[[code python]] … [[end]]               带语言名的代码块
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

## 6. 图片

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

## 7. 发布时的校验

| 情况 | 结果 |
|---|---|
| 两个页面路径相同 | 报错，指出冲突的路径 |
| `# nac:path` 为空或含 `..` / `?` / 竖线 | 报错，指出文件与原因 |
| Python 语法错 | 报错，带文件名与原因 |
| 文件带 BOM | 自动摘掉（Notepad 存的常见） |
| 静态页里用了会话/站点数据 | 报错，说明去掉 `# nac:static` 即可 |
| 图片扩展名不支持 | 报错，指出该转成什么 |
| `data` 或 `archive` 放在 `site` 里 | 报错（会把上一代产物当素材） |
| 静态页输出超过 16 KB | 报错 |

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