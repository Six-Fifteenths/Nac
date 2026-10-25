# nac:title 条目
# nac:match /hn/item/

# 一条故事 + 整棵评论树。
#
#   /hn/item/12        故事 12，评论框发给它
#   /hn/item/12/r7     故事 12，评论框是在回复第 7 条评论
#
# 地址的第二段决定「回复谁」，所以回复也是一条普通链接，不需要脚本。

import nac

from _lib import hn, hnview

db = hn.connect()
user = hn.current_user()

arg = nac.path_arg("/hn/item/")
head, _, rest = arg.partition("/")

hnview.header(db, user)

if not head.isdigit():
    nac.h2("没有这一条")
    nac.p("地址应该是 /hn/item/编号。")
    hnview.footer()
    nac.stop()

story_id = int(head)
reply_to = 0
if rest.startswith("r") and rest[1:].isdigit():
    reply_to = int(rest[1:])

story = hn.item(db, story_id)
if story is None or story["kind"] != hn.STORY:
    nac.h2("没有这一条")
    nac.p("第 %d 条不存在。" % story_id)
    nac.p(nac.inline_path("/hn/", "回首页"))
    hnview.footer()
    nac.stop()

hnview.story_head(db, story, user, nac.path())

kids, count = hn.thread(db, story_id)
nac.gap()
nac.h2("评论")
if not kids:
    nac.p(nac.i("还没有人评论。"))
else:
    nac.p(nac.i("%d 条评论，按时间排列。" % count))
    nac.gap()
    hnview.thread(kids, story_id)

nac.gap()
if user:
    hnview.compose(story_id, reply_to)
else:
    nac.p(nac.inline_path("/hn/login", "登录"), " 之后就能评论。")
hnview.footer()
