# nac:title 功能一览
# nac:path /features
# nac:static

import nac

nac.h1("功能一览")
nac.p("这一页本身就是一次预渲染：发布时执行 Python，把排版结果存成文件；访问时只读文件，不再启动解释器。")
nac.gap()

nac.h2("文本与强调")
nac.p("普通文本，", nac.b("粗体"), "，", nac.i("斜体"), "，", nac.tt("等宽字体"), "。")
nac.p("用户输入可以用 ", nac.tt("nac.esc()"), " 转义：", nac.esc("*这段不会变成粗体*"))
nac.p("段落中间也能插链接：见 ", "[[path:/notes/|笔记目录]]", " 与 ", "[[path:/sitemap|站点地图]]", "。")
nac.quote("引用块适合放一句短话；连续调用会合并成同一块。")
nac.gap()

nac.h2("列表")
with nac.cols(2):
    nac.h3("无序")
    nac.bullets("每项一行", "内容会转义", "可以很长")
    nac.col()
    nac.h3("有序")
    nac.ordered("编号自动加", "写什么数字都不影响", "渲染时重新数")
nac.gap()

nac.hr()
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

nac.h2("分栏里的容器")
with nac.cols(2):
    with nac.note("info", "左边"):
        nac.p("提示框里还能再分栏。")
    nac.col()
    with nac.note("ok", "右边"):
        nac.bullets("嵌套深度不限", "只要 [[end]] 对得上")
nac.gap()

nac.h2("图片")
nac.figure("img/protocol.png", "客户端与服务端之间的帧格式", 100)
nac.p("图片发布时只剪掉元数据段，像素不重编码；客户端拿到真实宽高，先预留位置。")
nac.gap()

nac.h2("底层标记")
nac.p("辅助函数不够用时，用 ", nac.tt("nac.raw()"), " 直接写一行标记：")
nac.code("[[path:/notes/|笔记目录]]")
nac.raw("[[path:/notes/|笔记目录]]")
nac.gap()

nac.h2("继续探索")
nac.nav("/", "/sitemap", "/gallery", "/interactive", "/toolbox")