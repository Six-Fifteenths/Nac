# nac:title 注册
# nac:path /register

"""注册：用户名 + 邮箱 + 密码。注册完直接登录。"""

import nac

from _lib import auth, store, ui

db = store.connect()
user = auth.current(db)

ui.head(user)

if user:
    nac.h1("已经登录了")
    nac.p("你现在是 ", nac.b(user["username"]), "。")
    nac.gap()
    nac.p(nac.inline_path("/", "回首页"))
    ui.footer()
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

nac.h1("注册")
nac.p(nac.i("一个用户名、一个邮箱、一个密码，够了。"))
nac.gap()
ui.errors(message)
nac.form_path("/register")
nac.field("username", "用户名")
nac.field("email", "邮箱")
nac.field("password", "密码", secret=True)
nac.gap()
nac.button("注册")
ui.hint("密码存在哪？", "SQLite 里存的是加盐 SHA-256，不是明文。")
ui.footer()
