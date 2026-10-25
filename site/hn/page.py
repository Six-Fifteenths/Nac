# nac:title Nac News 第 2 页起
# nac:match /hn/page/

# 首页的翻页。地址形如 /hn/page/3，页面自己从地址里取出页号。
# 一页认领一整个地址族，这是 Nac 里唯一的「路由」。

import nac

from _lib import hn, hnview

db = hn.connect()
user = hn.current_user()

total = hn.page_count(db)
try:
    page = int(nac.path_arg("/hn/page/") or "1")
except ValueError:
    page = 1
page = max(1, min(page, total))

hnview.header(db, user)
nac.p(nac.i("第 %d 页" % page))
nac.gap()
hnview.story_list(db, hn.ranked(db, page), page, total, user)
hnview.footer()
