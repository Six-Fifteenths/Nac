# nac:title 并排排版
# nac:static
# 演示 nac.cols()：同一个页面里做分栏。
# 不写 nac.col() 时按块数平均切；写了就按你指定的位置切。

import nac

nac.h1("并排排版")
nac.p("下面这一整块被分到两列里。")
nac.gap()

with nac.cols(2):
    nac.h2("左")
    nac.bullets("分栏是按块切的", "不是按像素切的", "两列高度不必相等")
    nac.col()
    nac.h2("右")
    nac.p("这半边继续写。", nac.b("粗体"), "、", nac.i("斜体"), "、", nac.tt("等宽"), "都能用。")
    nac.quote("窄列里引用要短。")

nac.gap()
nac.h2("三列")
with nac.cols(3):
    nac.p("第一列")
    nac.p("第二段还在第一列")
    nac.col()
    nac.p("第二列")
    nac.col()
    nac.p("第三列")

nac.gap()
nac.h2("按块数平均切（不写 nac.col）")
with nac.cols(2):
    nac.p("甲")
    nac.gap()
    nac.p("乙")
    nac.gap()
    nac.p("丙")
    nac.gap()
    nac.p("丁")

nac.gap()
nac.link(1, "回首页")

nac.done()
