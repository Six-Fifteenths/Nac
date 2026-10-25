"""登录状态：一个会话值存用户 id，别的什么都不存。"""

import nac

from _lib import store

KEY = "uid"


def current(db):
    """当前读者，没登录就是 None。"""
    uid = nac.session_get(KEY)
    if not uid:
        return None
    row = db.execute("select * from users where id=?", (uid,)).fetchone()
    if row is None:
        nac.session_pop(KEY)
        return None
    return row


def sign_in(user):
    nac.session_set(KEY, user["id"])


def sign_out():
    nac.session_pop(KEY)


def require(user, back="/"):
    """没登录就把人送去登录页。页面里紧跟一句 nac.stop()。"""
    if user is None:
        nac.p("这一页要先登录。")
        nac.gap()
        nac.link_path("/login", "去登录")
        nac.gap()
        nac.link_path(back, "回上一页")
        nac.stop()
