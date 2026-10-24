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
nac.quote("引用块适合放一句短话；连续调用会合并成同一块。")
nac.bullets(
    "无序列表：每项一行",
    "列表内容同样会转义",
    "行内样式可以拼在段落里",
)
nac.gap()

nac.h2("代码")
nac.code(
    "def greet(name):\n"
    "    return '你好，' + name\n"
    "\n"
    "nac.p(greet('读者'))"
)
nac.gap()

nac.h2("分栏")
with nac.cols(2):
    nac.h3("左栏")
    nac.p("分栏按块切分，不按像素；可以显式调用 ", nac.tt("nac.col()"), " 指定切点。")
    nac.bullets("两列", "三列", "最多四列")
    nac.col()
    nac.h3("右栏")
    nac.p("窄栏里适合放短段落、链接和小图。")
    nac.link_path("/layout", "看更多分栏")

nac.gap()
nac.h2("图片")
nac.img("img/protocol.png", 100)
nac.p("图片会在发布时重新编码并剥掉元数据；客户端拿到真实宽高，先预留位置，加载完不跳。")
nac.gap()

nac.h2("底层标记")
nac.p("如果辅助函数不够用，可以用 ", nac.tt("nac.raw()"), " 直接写一行标记：")
nac.code("[[path:/notes/|笔记目录]]")
nac.raw("[[path:/notes/|笔记目录]]")
nac.gap()

nac.h2("继续探索")
nac.link_path("/gallery", "图片走廊")
nac.link_path("/interactive", "互动实验")
nac.link_path("/notes/", "笔记目录")
nac.link_path("/", "回首页")
