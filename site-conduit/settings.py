# nac:title 设置
# nac:path /settings

"""设置：改邮箱、简介、密码；退出登录。"""

import nac

from _lib import auth, store, ui

db = store.connect()
user = auth.current(db)
ui.head(user)
auth.require(user, "/settings")

message = ""
done = False
if nac.submitted():
    message = store.update_user(
        db,
        user["id"],
        nac.field_of("email"),
        nac.field_of("bio"),
        nac.field_of("password"),
    )
    done = not message
    if done:
        user = store.user_by_name(db, user["username"])

nac.h1("设置")
nac.p(nac.i("改完按「保存」。密码留空就是不修改。"))
nac.gap()

if done:
    with nac.note("ok", "已保存"):
        nac.p("改动已经写进去。")
else:
    ui.errors(message)

nac.form_path("/settings")
nac.prefill("email", user["email"])
nac.prefill("bio", user["bio"])
nac.field("email", "邮箱")
nac.field("bio", "一句话简介")
nac.field("password", "新密码", secret=True)
nac.gap()
nac.button("保存")

nac.gap()
nac.hr()
nac.h2("退出")
nac.p(nac.inline_path("/logout", "退出登录"))
ui.footer()
