# nac:title Nac · 演示站
# nac:path /
# nac:static

import nac

nac.h1("一座不经过 HTTP 的小站")
nac.p("Nac 用自己的二进制 TCP 协议传输内容。页面是 Python 程序，客户端是 Rust 写的原生窗口。")
nac.p("没有 HTML，没有 JavaScript，也没有浏览器。")
nac.gap()
nac.quote("内容不是网页，但访问它的方式，应该像网页一样自然。")
nac.gap()
nac.img("img/beacon.png", 100)
nac.gap()

nac.h2("先逛一圈")
with nac.cols(3):
    nac.h3("排版")
    nac.p("标题、引用、列表、代码、分栏和图片。")
    nac.link_path("/features", "功能一览")
    nac.col()
    nac.h3("目录")
    nac.p("像普通网站一样，用路径进入目录首页。")
    nac.link_path("/notes/", "打开笔记目录")
    nac.col()
    nac.h3("互动")
    nac.p("表单、密码框、会话和站点私有数据。")
    nac.link_path("/interactive", "试一试")

nac.gap()
nac.h2("这个站点本身")
nac.bullets(
    "本代 %d 个页面、%d 张图片；页面路径由目录和文件名决定。" % (len(nac.PAGES), len(nac.IMAGES)),
    "首页和大部分说明页在发布时预渲染，访问时不再启动 Python。",
    "互动、留言和状态页每次访问都执行一次 Python，超时 2 秒。",
    "公开站使用 TLS，客户端固定证书；地址栏显示 nac.local，而不是服务器 IP。",
)
nac.gap()
nac.h2("更多入口")
nac.link_path("/gallery", "图片走廊")
nac.link_path("/layout", "分栏实验")
nac.link_path("/guestbook", "留言板")
nac.link_path("/status", "运行状态")
nac.link_path("/about", "关于 Nac")
