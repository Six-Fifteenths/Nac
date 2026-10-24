# nac:title 留言板
# nac:path /guestbook

import time

import nac

name = nac.field_of("name")[:40]
message = nac.field_of("message")[:200]

if message:
    if not name:
        name = "匿名"
    stamp = time.strftime("%Y-%m-%d %H:%M")
    nac.append_line("guestbook.txt", "%s · %s · %s" % (stamp, name, message))
    nac.session_set("name", name)

saved_name = nac.session_get("name")
recent_all = nac.read_lines("guestbook.txt")
count = len(recent_all)
recent = recent_all[-6:]

nac.h1("留言板")
nac.p("留言会追加到站点私有目录，下一次请求再从文件里读出来。")
nac.p("已有 ", nac.b(count), " 条留言。")
if saved_name:
    nac.p("你上次留下的名字：", nac.b(saved_name))
nac.gap()

if message:
    nac.quote("已记下。刷新页面就能在下面看到。")
    nac.gap()

if recent:
    nac.h2("最近的留言")
    for line in reversed(recent):
        nac.bullets(line)
else:
    nac.h2("最近的留言")
    nac.p("还没有人留言。")
nac.gap()

nac.h2("写一条")
nac.form_path("/guestbook")
nac.field("name", "你的名字（可留空）")
nac.field("message", "留言，最多 200 字")
nac.button("留下")
nac.gap()
nac.p("留言只存在这台服务器上；公开站没有账号，任何人提交的内容都请当作公开信息。")
nac.gap()
nac.link_path("/", "回首页")
nac.link_path("/status", "运行状态")
