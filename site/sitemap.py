# nac:title 站点地图
# nac:path /sitemap
# nac:static

import nac

nac.h1("站点地图")
nac.p("这一页没有手写任何一条链接：它在发布时执行，直接读本代的页面清单。")
nac.gap()

rows = [(("[[path:%s|%s]]" % (path, title or path)), nac.tt(path)) for _, path, title in nac.pages()]
nac.table(("页面", "路径"), rows)
nac.gap()

with nac.note("info", "为什么能这样"):
    nac.p("因为页面是 Python 程序，", nac.tt("nac.pages()"), " 在发布时就告诉你这一代有哪些页面、它们叫什么、路径是什么。")
    nac.p("加一个新页面，这一页会自动多一行，不用回来改。")

nac.gap()
nac.h2("统计")
nac.bullets(
    "本代页面 %d 个" % len(nac.PAGES),
    "图片 %d 张" % len(nac.IMAGES),
    "代名 %s" % nac.GEN,
)
nac.gap()
nac.link_path("/", "回首页")