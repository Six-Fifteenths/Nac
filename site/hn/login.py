# nac:title 登录
# nac:path /hn/login

import nac

from _lib import hn, hnview

db = hn.connect()
user = hn.current_user()

hnview.header(db, user)

if user:
    nac.h2("已经登录了")
    nac.p("现在用的是 %s。" % nac.inline_path("/hn/user/" + user, user))
    nac.p(nac.inline_path("/hn/logout", "退出"))
    hnview.footer()
    nac.stop()

name = nac.field_of("name")[:20]
pw = nac.field_of("password")
fresh = nac.checked("fresh")

if nac.submitted():
    if fresh:
        if hn.register(db, name, pw):
            hn.sign_in(name)
            nac.flash("账号建好了，%s。" % name)
            nac.redirect_path(nac.session_pop("next") or "/hn/")
            nac.stop()
        with nac.warn("没建成"):
            nac.p("这个名字要么是空的，要么已经有人用了。换一个试试。")
    elif hn.authenticate(db, name, pw):
        hn.sign_in(name)
        nac.flash("欢迎回来，%s。" % name)
        nac.redirect_path(nac.session_pop("next") or "/hn/")
        nac.stop()
    else:
        with nac.danger("密码不对"):
            nac.p("要么名字没注册过，要么密码错了。要新建账号就勾上下面那个框。")

nac.h2("登录")
nac.form_path("/hn/login")
nac.field("name", "用户名")
nac.password("password", "密码")
nac.check("fresh", "我是新来的，创建这个账号")
nac.button("进去")
nac.gap()
with nac.details("这里没有真正的安全性"):
    nac.p(
        "密码用 SHA-256 加盐后存在站点自己的 SQLite 里，验证用 "
        + nac.tt("secrets.compare_digest")
        + " 做定长比较。够挡顺手一试，不够挡真正的攻击者 —— 这是个复刻品，不是银行。"
    )
    nac.p(
        "会话放在服务端的会话文件里，客户端只拿一个 cookie 样的句柄，"
        "所以换个标签页就是另一段会话。"
    )
hnview.footer()
