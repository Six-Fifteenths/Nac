# nac:title 白塔
# 每个页面就是一个 Python 程序：拼好内容，nac.done() 输出。
# 页标题写在上面那行注释里，publish 会读走它。

import nac

nac.h1("白塔")
nac.p("第 3 号信标，仍然在广播。")
nac.gap()
nac.quote("他们把它留在这里，是因为它不会说话。")
nac.gap()
nac.bullets("东侧门：关闭", "西侧门：有人进去过")
nac.gap()
nac.img("img/plan.png", 80)
nac.gap()
nac.link(2, "第二页")
nac.link(7, "留言板")

nac.done()
