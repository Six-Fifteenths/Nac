# nac:title 一页看完
# nac:path /tour
# nac:static

"""排版能力一览：原来分在 features / layout / gallery / about 四页。

合并的理由很简单 —— 它们是同一件事的四段，分成四页只是让人多点三次。
这一页标了 `# nac:static`，所以它在发布时就执行完，访问时连 Python 都不启动。
"""

import nac

nac.h1("一页看完")
nac.p("这个平台能画出来的所有东西，都在这一页里。")
nac.p("它本身是一次预渲染：发布时执行 Python，把结果存成文件；访问时只读文件。")
nac.gap()

# ---------------------------------------------------------------- 文字
nac.h2("文字")
nac.p("普通文本，", nac.b("粗体"), "，", nac.i("斜体"), "，", nac.tt("等宽"), "。")
nac.p("用户输入用 ", nac.tt("nac.esc()"), " 转义：", nac.esc("*这段不会变成粗体*"))
nac.p("段落中间能插链接：见 ", nac.inline_path("/notes/", "笔记目录"), "。")
nac.quote("引用块适合放一句短话；连续调用会合并成同一块。")
nac.gap()

with nac.cols(2):
    nac.h3("无序")
    nac.bullets("每项一行", "内容会转义", "可以很长")
    nac.col()
    nac.h3("有序")
    nac.ordered("编号自动加", "写什么数字都不影响", "渲染时重新数")
nac.gap()

# ---------------------------------------------------------------- 结构
nac.h2("表格")
nac.table(
    ("容器", "能放什么", "能嵌套吗"),
    [
        (nac.tt("cols"), "任意块", "能"),
        (nac.tt("note"), "任意块", "能"),
        (nac.tt("details"), "任意块", "能"),
        (nac.tt("tabs"), "任意块", "能"),
        (nac.tt("table"), "只放文字与行内样式", "不能"),
    ],
)
nac.p("第一行是表头；单元格里能用粗体、等宽和行内链接。")
nac.gap()

nac.h2("提示框")
with nac.tip("提示"):
    nac.p("蓝色框，放补充说明。")
with nac.ok("成功"):
    nac.p("绿色框，放「操作完成」这类信息。")
with nac.warn("注意"):
    nac.p("黄色框，放容易踩的坑。")
with nac.danger("危险"):
    nac.p("红色框，放不可逆的操作警告。")
nac.gap()

nac.h2("折叠与页签")
with nac.details("点我展开"):
    nac.p("默认收起。适合放「想看再看」的长内容、实现细节、日志。")
    nac.bullets("里面可以放列表", "也可以放代码")
with nac.tabs("第一个页签", "第二个页签"):
    nac.p("同一块地方，切换着看。选中状态由客户端记住。")
    nac.tab()
    nac.p("换页再回来，还停在你选的那个页签上。")
nac.gap()

nac.h2("代码")
nac.code(
    "def greet(name):\n"
    "    return '你好，' + name\n"
    "\n"
    "nac.table((\"列\",), [(greet('读者'),)])",
    lang="python",
)
nac.gap()

# ---------------------------------------------------------------- 分栏与盒子
nac.h2("分栏")
nac.p("没有网格、没有浮动，只有一种按块切分的分栏；每列宽由客户端平分。")
with nac.cols(2):
    with nac.note("info", "左边"):
        nac.p("提示框里还能再分栏。")
    nac.col()
    with nac.note("ok", "右边"):
        nac.bullets("嵌套深度不限", "只要 [[end]] 对得上")
nac.gap()
with nac.cols(2):
    nac.h3("左：说明")
    nac.p("分栏只决定块的顺序和宽度，不测量文字高度。每个块仍然是独立的段落、列表或链接。")
    nac.col()
    nac.img("img/palette.png", 100)
nac.p(nac.i("抽象色块，生成于发布前。"))
nac.gap()

# ---------------------------------------------------------------- 图片
nac.h2("图片")
nac.p("图片不是外链，而是站点目录里的文件；发布时只剪掉元数据段，像素不重编码。")
nac.figure("img/protocol.png", "客户端与服务端之间的帧格式", 100)
with nac.cols(2):
    nac.img("img/palette.png", 100)
    nac.p(nac.i("palette.png"), "：半透明色块叠在一起。")
    nac.col()
    nac.img("img/beacon.png", 100)
    nac.p(nac.i("beacon.png"), "：灯塔、信号和一点点噪声。")
nac.gap()

# ---------------------------------------------------------------- 底层
nac.h2("底层标记")
nac.p("辅助函数不够用时，用 ", nac.tt("nac.raw()"), " 直接写一行标记：")
nac.code("[[path:/notes/|笔记目录]]")
nac.raw("[[path:/notes/|笔记目录]]")
nac.gap()

# ---------------------------------------------------------------- 关于
nac.h2("这是什么")
nac.p("Nac 是一个自成一体的发布平台：自己的二进制 TCP 协议、自己的页面格式、自己的客户端。")
nac.p("它不是为了取代网页，而是想验证一条更窄的路：如果内容只在专用客户端里阅读，能省掉多少层？")
nac.gap()
nac.quote("一个页面就是一个 Python 程序；客户端只负责把程序输出的标记排成可读的版面。")
nac.gap()
nac.bullets(
    "没有 HTML、CSS、JavaScript，也没有浏览器引擎。",
    "协议很小：一请求一响应，帧上限 32 KB，页面输出上限 16 KB。",
    "页面脚本超时 2 秒，失败统一返回空响应，不泄露失败原因。",
    "发布是原子的：新代写完才切换 current，失败不会留下半套内容。",
    "公开站使用 TLS 和客户端证书固定；运营台只监听回环。",
)
nac.gap()

with nac.details("这个站点自己用了什么"):
    nac.table(
        ("部分", "做法"),
        [
            ("传输", "u32 长度 + 类型 + 负载，长度上限 32 KB"),
            ("渲染", "Rust + egui 原生窗口，不是浏览器内核"),
            ("页面", "Python 程序，标准库随便用"),
            ("存储", "站点私有目录里的 JSON 与 SQLite"),
            ("加密", "公开站 TLS + 证书固定；运营台只监听回环"),
        ],
    )

nac.gap()
nac.hr()
nac.nav("/", "/tour", "/guestbook", "/hanoi", "/sitemap")
