# nac:title 登录
# nac:path /login

"""登录：邮箱 + 密码。成功后回到首页。"""

import nac

from _lib import auth
from _lib import store
from _lib import ui

db = store.connect()
user = auth.current(db)

if user:
    nac.h1("已经登录了")
    nac.p("你现在是 ", nac.b(user["username"]), "。")
    nac.gap()
    nac.link_path("/", "回首页")
    nac.gap()
    nac.link_path("/logout", "退出登录")
    nac.stop()

message = ""
if nac.submitted():
    row = store.login(db, nac.field_of("email"), nac.field_of("password"))
    if row is None:
        message = "邮箱或密码不对。"
    else:
        auth.sign_in(row)
        nac.redirect_path("/")

ui.nav(None)
nac.h1("登录")
ui.errors(message)
nac.form_path("/login")
nac.field("email", "邮箱")
nac.field("password", "密码", secret=True)
nac.gap()
nac.button("登录")
nac.gap()
nac.p("还没有账号？", nac.inline_path("/register", "注册一个"))
