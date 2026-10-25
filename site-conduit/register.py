# nac:title 注册
# nac:path /register

"""注册：用户名 + 邮箱 + 密码。注册完直接登录。"""

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
    nac.stop()

message = ""
if nac.submitted():
    row, message = store.create_user(
        db,
        nac.field_of("username"),
        nac.field_of("email"),
        nac.field_of("password"),
    )
    if row is not None:
        auth.sign_in(row)
        nac.redirect_path("/")

ui.nav(None)
nac.h1("注册")
ui.errors(message)
nac.form_path("/register")
nac.field("username", "用户名")
nac.field("email", "邮箱")
nac.field("password", "密码", secret=True)
nac.gap()
nac.button("注册")
nac.gap()
nac.p("已经有账号？", nac.inline_path("/login", "去登录"))
