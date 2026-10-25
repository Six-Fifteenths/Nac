# nac:title 最新
# nac:path /hn/newest

import nac

from _lib import hn, hnview

db = hn.connect()
user = hn.current_user()

total = hn.page_count(db)
hnview.header(db, user)
nac.p("按时间倒序，不按分数。新东西先出现，哪怕只有 1 分。")
nac.gap()
hnview.story_list(db, hn.newest(db, 1), 1, total, user)
hnview.footer()
