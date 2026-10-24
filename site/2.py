# nac:title 第二页
# 这一页顺便演示：页面脚本可以读环境、算东西，再把结果写进排版。

import nac

title = "第二页"
nac.h1(title)
nac.p("这是", nac.b("第 3 页"), "之外的另一个页面，用来验证链接与排版。")
nac.gap()
nac.quote("每个页面都是一次独立的 Python 调用；没有常驻状态，也没有内存里的变量。")
nac.gap()
nac.h2("这一页能做的事")
nac.bullets(
    "读环境变量、读文件、连数据库",
    "根据请求里的表单字段决定显示什么",
    "把结果直接拼进排版",
)

if nac.request().get("fields"):
    nac.gap()
    nac.p("这次是带着表单数据进来的。")

nac.gap()
nac.link(1, "回首页")

nac.done()
