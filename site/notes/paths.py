# nac:title 路径与目录
# nac:path /notes/paths
# nac:static

import nac

nac.h1("路径与目录")
nac.p("Nac 的地址像普通网站：域名只是显示，路径决定打开哪一页。")
nac.gap()

nac.h2("文件就是路径")
nac.table(
    ("文件", "路径", "说明"),
    [
        (nac.tt("site/index.py"), nac.tt("/"), "站点首页"),
        (nac.tt("site/features.py"), nac.tt("/features"), "普通页面"),
        (nac.tt("site/notes/index.py"), nac.tt("/notes/"), "目录首页"),
        (nac.tt("site/notes/paths.py"), nac.tt("/notes/paths"), "目录里的页面"),
    ],
)
nac.p("目录首页必须叫 ", nac.tt("index.py"), "；没有它的目录不会自动列出内容，访问时返回 404。")
nac.gap()

nac.h2("覆盖路径")
nac.p("如果文件名不方便直接当路径，可以在页面靠前位置写：")
nac.code("# nac:path /custom/path", lang="python")
with nac.note("warn", "路径必须唯一"):
    nac.p("同一代里两条路径相同会让发布直接失败，报错会指出冲突的是哪两个文件。")
nac.gap()

nac.h2("链接与表单")
nac.code(
    "nac.link_path('/notes/', '笔记目录')\n"
    "nac.form_path('/guestbook')\n"
    "nac.redirect_path('/')",
    lang="python",
)
nac.p("这些函数会在发布时把路径映射成内部页号；写页面时不用关心数字。")
nac.gap()
nac.nav("/", "/notes/", "/notes/paths")