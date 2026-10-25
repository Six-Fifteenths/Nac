# nac:title 最新的第 2 页起
# nac:match /hn/newest/

# /hn/newest/2、/hn/newest/3 …… 按时间翻页。
import nac

from _lib import hn, hnview

db = hn.connect()
user = hn.current_user()

total = hn.page_count(db)
try:
    page = int(nac.path_arg("/hn/newest/") or "1")
except ValueError:
    page = 1
page = max(1, min(page, total))

hnview.header(db, user)
nac.p(nac.i("最新 · 第 %d 页" % page))
nac.gap()
hnview.story_list(db, hn.newest(db, page), page, total, user)
hnview.footer()
