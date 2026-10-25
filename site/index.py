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
nac.figure("img/beacon.png", "本代 %d 个页面，%d 张图片" % (len(nac.PAGES), len(nac.IMAGES)), 100)
nac.gap()

nac.h2("先逛一圈")
with nac.cols(3):
    nac.h3("排版")
    nac.p("标题、列表、表格、提示框、折叠、页签、分栏，一页看完。")
    nac.link_path("/tour", "一页看完")
    nac.col()
    nac.h3("会动的")
    nac.p("汉诺塔：三根柱子、可选层数、一次真正的移动模拟。")
    nac.link_path("/hanoi", "搭起来看看")
    nac.col()
    nac.h3("真的能填")
    nac.p("留言板：SQLite、参数化查询、会话与跳转。")
    nac.link_path("/guestbook", "留一句")

nac.gap()
nac.h2("每一页都是程序")
with nac.cols(2):
    with nac.note("info", "静态页"):
        nac.p("发布时执行一次，结果存成文件。访问时连 Python 都不启动。")
        nac.p("首页、这一页，还有排版那一页，都是静态页。")
    nac.col()
    with nac.note("ok", "动态页"):
        nac.p("每次访问都真的跑一次 Python，可以读时间、读表单、读写数据库。")
        nac.p("超时 2 秒，失败只回空响应。")

nac.gap()
with nac.details("这个站点本身用了什么"):
    nac.table(
        ("部分", "做法"),
        [
            ("传输", "u32 长度 + 类型 + 负载，长度上限 32 KB"),
            ("渲染", "Rust + egui 原生窗口，不是浏览器内核"),
            ("页面", "Python 程序，标准库随便用"),
            ("存储", "站点私有目录里的 JSON 与 SQLite"),
            ("加密", "公开站 TLS + 证书固定；运营台只监听回环"),
        ],
    )

nac.gap()
nac.table(
    ("页面", "看什么"),
    [
        ("[[path:/tour|一页看完]]", "所有排版能力"),
        ("[[path:/hanoi|汉诺塔]]", "服务端算 2ⁿ−1 步，客户端按关键帧动画"),
        ("[[path:/guestbook|留言板]]", "SQLite、参数化查询、会话与跳转"),
        ("[[path:/notes/|笔记]]", "路径与目录、标记语法"),
        ("[[path:/sitemap|站点地图]]", "发布时自动生成的页面清单"),
    ],
)
nac.gap()
nac.link_path("/", "回首页")
