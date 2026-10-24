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
- 内部页号仍然存在（协议要用 `u16`），但**写页面时不要依赖它**：数字文件名保留自己的页号，
  命名文件由 `publish` 分配稳定 id。页面之间请用 `nac.link_path()` 链接。
- **页名**来自 `# nac:title <文字>`，`publish` 读走写进清单；没有就留空，客户端显示路径。
- `import nac` 不需要安装：每代发布都会在 `scripts/` 里生成它。
- `nac.done()` 输出页面。忘了写也没关系——进程退出时会自动输出已拼好的内容。

---

## 1. nac 函数全表

生成的 `nac.py` 按用途分成 11 节，**只有 C/D/E/F/G 会往页面上写内容**，其余都是读环境或工具函数。

### A. 本代常量

| 名字 | 内容 |
|---|---|
| `nac.PAGES` | 本代所有内部页号，如 `(1, 2, 3, 7)`（一般不用） |
| `nac.PATHS` | 路径到内部页号的映射，如 `{"/": 1, "/about": 2}`（一般不用） |
| `nac.IMAGES` | 图片表，如 `{"img/plan.png": (1, 640, 360)}`（id, 宽, 高） |

### B. 文本与转义

| 调用 | 说明 |
|---|---|
| `nac.esc(text)` | 转义用户输入，使其不会被当成标记 |
| `nac.b(text)` | 粗体 |
| `nac.i(text)` | 斜体 |
| `nac.tt(text)` | 等宽 |

### C. 块级排版

| 调用 | 说明 |
|---|---|
| `nac.h1/h2/h3(text)` | 三级标题 |
| `nac.p(*parts)` | 段落，参数按顺序拼接 |
| `nac.quote(*parts)` | 引用（连续调用合成一块） |
| `nac.bullets(*items)` | 无序列表，一项一行 |
| `nac.code(text)` | 等宽块，保留换行 |
| `nac.gap()` | 空行，分段与间距 |
| `nac.raw(line)` | 原样写一行线上标记，兜底用 |

### D. 布局

| 调用 | 说明 |
|---|---|
| `with nac.cols(n=2):` | 里面的内容并排成 n 列（最多 4） |
| `nac.col()` | 在 `cols` 里强制换到下一列 |

### E. 媒体

| 调用 | 说明 |
|---|---|
| `nac.img(path, width=None)` | 插图片；`path` 是 `site/` 里的相对路径，`width` 是百分比 |

### F. 导航

| 调用 | 说明 |
|---|---|
| `nac.link_path(path, text)` | 跳到路径；目录要带尾斜杠（如 `/notes/`） |
| `nac.link(page, text)` | 按内部页号跳转；页号不存在时整行不显示 |
| `nac.redirect_path(path)` | 按路径立刻跳转；路径不存在时不动 |
| `nac.redirect(page)` | 按内部页号跳转，本页已拼的内容丢弃 |
| `nac.first_page()` | 本代最小内部页号（一般用 `link_path("/")`） |

### G. 表单

| 调用 | 说明 |
|---|---|
| `nac.form_path(path)` | 声明表单提交到某个路径 |
| `nac.form(page)` | 声明表单提交给内部页号 |
| `nac.field(name, label="", secret=False)` | 单行输入 |
| `nac.password(name, label="")` | 密码输入（`field(..., secret=True)` 的简写） |
| `nac.button(label)` | 提交按钮 |

### H. 请求

| 调用 | 说明 |
|---|---|
| `nac.request()` | `{"page": 页号, "fields": {...}}` |
| `nac.fields()` | 只要字段字典 |
| `nac.field_of(name, default="")` | 取一个字段并去掉首尾空白 |
| `nac.page_id()` | 当前页号 |

### I. 会话

| 调用 | 说明 |
|---|---|
| `nac.session()` | 会话状态字典（跨页、跨请求） |
| `nac.save_session(state)` | 写回状态 |
| `nac.session_get(key, default=None)` | 读一个键 |
| `nac.session_set(key, value)` | 写一个键（内部已保存） |

### J. 站点数据（`data/appdata/`）

| 调用 | 说明 |
|---|---|
| `nac.data_dir()` | 私有目录路径（自动创建） |
| `nac.data_path(name)` | 私有目录里的某个文件 |
| `nac.read_json(name, default=None)` | 读 JSON；不存在或坏了返回默认值 |
| `nac.write_json(name, value)` | **原子**写 JSON（先 `.tmp` 再替换） |
| `nac.read_lines(name)` | 读文本行；不存在返回 `[]` |
| `nac.append_line(name, text)` | 追加一行 |
| `nac.counter(name, step=1)` | 计数并返回新值（首次从 0 起） |

### K. 输出

| 调用 | 说明 |
|---|---|
| `nac.done()` / `nac.flush()` | 输出页面（不调用也会在退出时自动输出） |

`nac` 里所有写文件的地方都是**原子写**，断电或并发不会留下半截 JSON。
`field/button/link` 会自动清掉标记里的分隔符（`|`、`]]`），所以你不用手动防注入式破坏排版。

---

## 2. 完整例子：留言板

`site/7.py`（仓库里就是这个），用到了表单、会话与站点私有目录里的小工具：

```python
# nac:title 留言板
import nac

name = nac.field_of("name")[:40]
if name:
    nac.session_set("name", name)
    nac.append_line("guestbook.txt", name)
    nac.counter("visits.json")

nac.h1("留言板")
nac.p("你上次留下的名字：", nac.session_get("name") or "（还没有）")
nac.p("本页被提交过 ", nac.b(nac.read_json("visits.json", {"n": 0})["n"]), " 次")
nac.gap()

recent = nac.read_lines("guestbook.txt")[-5:]
if recent:
    nac.quote("最近的记录")
    nac.bullets(*recent)
else:
    nac.p("还没有人留言。")

nac.gap()
nac.form(7)
nac.field("name", "你的名字")
nac.button("留下")
nac.gap()
nac.link(1, "回首页")

nac.done()
```

要点：**页面自己负责一切**——读文件、写文件、判断是否需要跳转。没有框架，也没有中间层。
判断"这是不是一次表单提交"用 `if name:`（`field_of` 已去空白），
不要用 `if nac.fields():`——空输入框也会作为空字符串提交上来。

---

## 3. 线上标记（`nac` 最终输出的东西）

客户端只认这套行标记；`nac` 是生成它们的便捷方式，`nac.raw()` 可以直接写。

```
# 标题          ## 二级        ### 三级
空行            分隔段落
> 引用          （连续的 > 行会合成一个引用块）
- 列表项        （连续的 - 行会合成一个列表）
| 等宽行        （连续的 | 行会合成一个代码块）
!img 1 80% 640x360     图片：id、宽度%、可选原始宽高比
[[2|第二页]]            页内跳转
[[form 7]]             表单提交目标
[[input name|提示]]     单行输入
[[secret pass|提示]]    密码输入
[[button 提交]]         按钮
[[cols 2]] … [[end]]   并排布局
[[col]]                在 cols 里换列
```

行内：`*粗*`、`_斜_`、`` `等宽` ``，反斜杠 `\` 转义；`\n` 在文本里会被压成空格。

**段落规则**：连续的普通行会合并成一个段落（用空格连接）。想断开就在中间放一个空行，
或者用 `nac.gap()`。

---

## 4. 分栏

```python
with nac.cols(2):
    nac.h2("左")
    nac.bullets("第一项", "第二项")
    nac.col()                      # 换到下一列
    nac.h2("右")
    nac.p("右边的内容。")
```

- 不写 `nac.col()` 时，按**块数**平均分配到 n 列。
- 写了 `nac.col()`，切点完全由你决定，`n` 会被忽略（列数 = 切点 + 1）。
- 最多 4 列。列宽由客户端按可用宽度平分；两列高度不会自动对齐。
- 可以嵌套：`with nac.cols(2)` 里再 `with nac.cols(2)`。

---

## 5. 图片

1. 把图片放进 `site/`（例如 `site/img/plan.png`），**直接放原始文件，不要为了发布另存一次**。
2. 页面里 `nac.img("img/plan.png", 80)`（`80` 是宽度百分比，省略即 100%）。
3. 发布时：
   - 支持 `png` / `jpeg` / `webp` / `gif`；
   - **不重编码**，只按容器结构剪掉元数据段，像素逐位不变：

     | 格式 | 丢掉 | 保留 |
     |---|---|---|
     | PNG | `tEXt` `iTXt` `zTXt` `tIME` `eXIf` | 其余全部 |
     | JPEG | `APP1`(EXIF/XMP) `APP12` `APP13`(IPTC) `COM` | APP0/APP2(ICC)/APP14 + 图像数据 |
     | WebP | `EXIF` `XMP ` 块（并清掉 VP8X 标志位） | 其余块 |
     | GIF | Comment Extension | 其余块 |

   - `bmp / tif / heic / avif` 会被拒绝，要求先转换；带 XMP 的 GIF 也会被拒绝
     （XMP 藏在带魔数尾的 sub-block 里，猜错会毁图）。
   - 宽高会被读出并写进 `nac.py`，客户端据此在图片到达前就预留正确高度。
4. 想保留带元数据的原图：`publish --archive <目录>`，发布前先把原件拷过去。
   归档目录不要放在 `site/` 里（会被当成新素材再发一次，`publish` 会直接拒绝）。

已知取舍：EXIF 里的 `Orientation` 会被一起去掉。客户端目前不读这个标签，
所以竖拍照片的旋转问题剥离前剥离后一样——真要修，得把该标签单独提出来传给客户端。

---

## 5.5 静态页

页面里加一行 `# nac:static`（放在标题附近即可），`publish` 会在发布时执行一次页面，
把输出存到该代的 `static/<id>.txt`。线上由 `contentd` 直接读文件，不再起 Python。

- 适合不依赖表单/会话/请求的页面
- 输出同样受 16 KB 限制；执行失败会让整个发布失败
- 静态页的元数据 `kind` 是 `static`，动态页仍是 `dynamic`
- 静态页发布时以发布者身份执行，不要在里面写有副作用的代码

## 6. 发布时的校验

| 情况 | 结果 |
|---|---|
| 两个页面路径相同 | 报错，指出冲突的路径 |
| `# nac:path` 为空或含 `..` / `?` / `|` 等字符 | 报错，指出文件与原因 |
| 存在 `.page` 文件 | 报错并提示改名 |
| Python 语法错 | 报错，带文件名与原因（`2.py: SyntaxError: '(' was never closed`） |
| 文件带 BOM | 自动摘掉（Notepad 存的文件常见） |
| 图片扩展名不支持 | 报错，指出该转成什么 |
| `data` 或 `archive` 放在 `site` 里 | 报错（会把上一代产物/归档原件当成素材） |
| 引用了不存在的图片路径 | 运行时 `nac.img()` 抛 `KeyError`，服务端返回 `EMPTY` 并在 stderr 记录 |
| 引用了不存在的页号 | `nac.link()` 静默跳过，页面上不出现这个链接 |
| 引用了不存在的路径 | `nac.link_path()` 仍会显示，点击后是 404 |

发布失败时**旧的一代继续服务**，不会出现"发了一半"的状态：新代写完才原子更新 `data/current`。

---

## 7. 运行环境契约

`publish` 只做静态检查；页面在 `contentd` 里执行，契约是环境变量 + stdout：

| 环境变量 | 含义 |
|---|---|
| `NAC_PAGE` | 当前页号 |
| `NAC_INPUT` | JSON：`{"page": N, "fields": {...}}` |
| `NAC_SESSION_FILE` | 会话状态文件路径（`nac.session()` 读写它） |
| `NAC_DATA_DIR` | 站点私有目录（`nac.data_dir()` 返回它） |
| 其他 | `contentd --env K=V` 传进来的额外变量（运营台用 `NAC_ROOT`） |

- 进程以 `python3 -B -X utf8` 启动，工作目录是代目录，`PYTHONIOENCODING=utf-8`。
- **stdout 就是页面内容**；第一个非空白行若是 `@@REDIRECT <页号>`，服务端改发那一页。
- stderr 只进服务端日志，不会到客户端。
- 限制：超时 2 秒、stdout ≤ 128 KB、页面 ≤ 16 KB；任何失败都回 `EMPTY`。
- 没有常驻内存：每次请求都是新进程，状态只能放 `data_dir()` 或会话文件。
