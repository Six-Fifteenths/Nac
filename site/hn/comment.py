# nac:title 发布评论
# nac:path /hn/comment

# 表单提交的目标页。插一条评论，回到故事页。

import time

import nac

from _lib import hn

db = hn.connect()
user = hn.current_user()

story = nac.field_of("story")
parent = nac.field_of("parent")
text = nac.field_of("text")[:2000]

story_id = int(story) if story.isdigit() else 0

if not user:
    nac.redirect_path("/hn/login")
elif not story_id or not text:
    nac.redirect_path("/hn/item/%d" % story_id if story_id else "/hn/")
else:
    parent_id = int(parent) if parent.isdigit() else 0
    # 父编号只认属于同一个故事的评论；地址被拼歪了也只会退回成顶层评论。
    if parent_id and hn.root_of(db, parent_id) != story_id:
        parent_id = 0
    db.execute(
        "insert into item(kind, parent, body, author, at) values(?, ?, ?, ?, ?)",
        (hn.COMMENT, parent_id or story_id, text, user, int(time.time())),
    )
    db.commit()
    nac.flash("评论发出去了。")
    nac.redirect_path("/hn/item/%d" % story_id)
