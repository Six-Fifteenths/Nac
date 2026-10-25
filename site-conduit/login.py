# nac:title 登录
# nac:path /login

"""登录：邮箱 + 密码。成功后回到首页。"""

import nac

from _lib import auth, store, ui

db = store.connect()
user = auth.current(db)

ui.head(user)

if user:
    nac.h1("已经登录了")
    nac.p("你现在是 ", nac.b(user["username"]), "。")
    nac.gap()
    nac.p(nac.inline_path("/", "回首页"), "  ·  ", nac.inline_path("/logout", "退出登录"))
    ui.footer()
    nac.stop()

message = ""
if nac.submitted():
    row = store.login(db, nac.field_of("email"), nac.field_of("password"))
    if row is None:
        message = "邮箱或密码不对。"
    else:
        auth.sign_in(row)
        nac.redirect_path("/")

nac.h1("登录")
nac.p(nac.i("欢迎回来。"))
nac.gap()
ui.errors(message)
nac.form_path("/login")
nac.field("email", "邮箱")
nac.field("password", "密码", secret=True)
nac.gap()
nac.button("登录")
ui.hint("忘记密码？", "这里没有找回流程：重新注册一个账号，或者忘掉它。")
ui.footer()
