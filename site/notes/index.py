# nac:title 笔记
# nac:path /notes/
# nac:static

import nac

nac.h1("笔记")
nac.p("这是 /notes/ 目录的首页，对应文件 site/notes/index.py。")
nac.p("目录本身不列文件；访问 /notes/ 会进入这个 index 页面，其他页面要显式链接。")
nac.gap()

nac.h2("目录里有什么")
nac.bullets(
    "路径与目录：文件名、index.py 和 # nac:path 的关系。",
    "页面标记：客户端真正认识的底层行标记。",
)
nac.gap()

nac.h2("两个子页面")
nac.link_path("/notes/paths", "路径与目录")
nac.link_path("/notes/markup", "页面标记")
nac.gap()
nac.p("本代共有 ", nac.b(len(nac.PAGES)), " 个页面。")
nac.gap()
nac.link_path("/", "回首页")
nac.link_path("/features", "功能一览")
