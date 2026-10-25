# nac:title 留言板
# nac:path /guestbook

import time

import nac

db = nac.db()
db.execute(
    "create table if not exists msg("
    "id integer primary key, name text, body text, at text)"
)

name = nac.field_of("name")[:40]
body = nac.field_of("message")[:200]

accepted = False
if body and nac.checked("public"):
    db.execute(
        "insert into msg(name, body, at) values(?, ?, ?)",
        (name or "匿名", body, time.strftime("%Y-%m-%d %H:%M")),
    )
    db.commit()
    nac.session_set("name", name or "匿名")
    nac.flash("留言已记下")
    nac.redirect_path("/guestbook")
elif body:
    accepted = False

rows = db.execute("select name, body, at from msg order by id desc limit 12").fetchall()
total = db.execute("select count(*) as n from msg").fetchone()["n"]
saved = nac.session_get("name")

nac.h1("留言板")
nac.p("留言写进站点私有目录里的 SQLite 数据库：不联网，也不需要额外的数据库服务。")

for line in nac.flashes():
    with nac.note("ok", "刚完成"):
        nac.p(line)

if body and not nac.checked("public"):
    with nac.note("warn", "还没记下"):
        nac.p("要勾一下「我知道留言是公开的」才会保存。")

nac.gap()
with nac.cols(3):
    nac.h3("留言总数")
    nac.p(nac.b(total))
    nac.col()
    nac.h3("你的名字")
    nac.p(nac.b(saved or "（还没有）"))
    nac.col()
    nac.h3("存储位置")
    nac.p(nac.tt("data/appdata/site.db"))

nac.gap()
if rows:
    nac.h2("最近 12 条")
    nac.table(
        ("名字", "留言", "时间"),
        [(row["name"], row["body"], nac.tt(row["at"])) for row in rows],
    )
else:
    nac.p("还没有人留言。")

nac.gap()
nac.h2("写一条")
nac.form_path("/guestbook")
nac.field("name", "你的名字（可留空）")
nac.textarea("message", "留言，最多 200 字")
nac.check("public", "我知道留言是公开的")
nac.button("留下")
nac.gap()

with nac.details("这一页做了什么"):
    nac.ordered(
        "打开（或新建）data/appdata/site.db，建表",
        "把表单内容用参数化 SQL 插进去，然后 commit",
        "把提示信息塞进会话，跳回同一页",
        "重新查最近 12 条，拼成表格发回客户端",
    )
    nac.p("参数化 SQL 意味着留言内容不会被当成 SQL 执行；文本永远只是文本。")

nac.gap()
nac.nav("/", "/guestbook", "/status")