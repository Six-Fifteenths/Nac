# nac:title 分栏实验
# nac:path /layout
# nac:static

import nac

nac.h1("分栏实验")
nac.p("Nac 没有 CSS 网格，也没有浮动；只有一种按块切分的分栏。")
nac.p("它不追求像素级控制，只保证窄窗口里仍然能读。")
nac.gap()

nac.h2("文字 + 图片")
with nac.cols(2):
    nac.h3("左：说明")
    nac.p("这一栏放文字。分栏只决定块的顺序和宽度，不会测量文字高度。")
    nac.p("每个块仍然是独立的段落、列表或链接。")
    nac.gap()
    nac.link_path("/features", "回功能一览")
    nac.col()
    nac.img("img/palette.png", 100)
    nac.p(nac.i("抽象色块，生成于发布前。"))

nac.gap()
nac.h2("三栏卡片")
with nac.cols(3):
    nac.h3("快")
    nac.p("静态页发布时预渲染，访问时不启动 Python。")
    nac.col()
    nac.h3("小")
    nac.p("页面输出上限 16 KB，图片按 16 KB 分片流式传输。")
    nac.col()
    nac.h3("直")
    nac.p("自己的 TCP 协议，出错时统一返回空响应。")

nac.gap()
nac.h2("嵌套分栏")
with nac.cols(2):
    nac.h3("外框左")
    with nac.cols(2):
        nac.p("内层甲")
        nac.col()
        nac.p("内层乙")
    nac.col()
    nac.h3("外框右")
    nac.p("嵌套只建议放短内容；窗口变窄时列会显得挤。")

nac.gap()
nac.p("分栏的源码也很直白：")
nac.code(
    "with nac.cols(3):\n"
    "    nac.h3('第一栏')\n"
    "    nac.p('...')\n"
    "    nac.col()\n"
    "    nac.h3('第二栏')\n"
    "    nac.col()\n"
    "    nac.h3('第三栏')"
)
nac.gap()
nac.link_path("/gallery", "图片走廊")
nac.link_path("/", "回首页")
