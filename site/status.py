# nac:title 运行状态
# nac:path /status

import json
import time

import nac

views = nac.counter("status-views.json")

db = nac.db()
has_msg = db.execute(
    "select name from sqlite_master where type='table' and name='msg'"
).fetchone()
messages = db.execute("select count(*) as n from msg").fetchone()["n"] if has_msg else 0
saved_name = nac.session_get("name")
saved_mood = nac.session_get("mood")
gen = nac.GEN
pages = nac.pages()
manifest = json.loads(
    (nac.data_dir().parent / "gen" / gen / "manifest.json").read_text(encoding="utf-8")
)
stats = {}
for page in manifest.get("pages", []):
    stats[page["kind"]] = stats.get(page["kind"], 0) + 1

nac.h1("运行状态")
nac.p("这一页每次访问都执行 Python，读当前时间、当前代和站点私有数据。")
nac.gap()

with nac.cols(2):
    nac.h3("这次请求")
    nac.p("服务端时间：", nac.tt(time.strftime("%Y-%m-%d %H:%M:%S")))
    nac.p("时区：", nac.tt(time.strftime("%Z") or "本地"))
    nac.p("状态页已看过 ", nac.b(views), " 次。")
    nac.col()
    nac.h3("这一代")
    nac.p("代名：", nac.tt(gen))
    nac.p("页面 ", nac.b(len(pages)), " 个（静态 ", stats.get("static", 0), " / 动态 ", stats.get("dynamic", 0), "）。")
    nac.p("图片 ", nac.b(len(nac.IMAGES)), " 张。")

nac.gap()
nac.h2("站点数据")
nac.table(
    ("数据", "位置", "现状"),
    [
        ("留言", nac.tt("site.db · msg"), "%d 条" % messages),
        ("状态页计数", nac.tt("status-views.json"), "%d 次" % views),
        ("会话", nac.tt("state/sessions/"), "名字 %s" % nac.b(saved_name or "（空）")),
    ],
)
with nac.tip("都在这台机器上"):
    nac.p("上面三个都在 ", nac.tt("data/appdata/"), " 与 ", nac.tt("data/state/"), " 里，不外发，也不联网。")

nac.gap()
nac.h2("这一代有哪些页")
nac.table(
    ("页面", "路径"),
    [("[[path:%s|%s]]" % (path, title or path), nac.tt(path)) for _, path, title in pages],
)

nac.gap()
nac.h2("观察点")
nac.bullets(
    "静态页的访问不会出现在 Python 启动记录里；动态页会。",
    "内容更新只需要重新发布，不用重启 contentd。",
    "页面失败统一返回空响应，原因只写服务端日志。",
)
nac.gap()
nac.nav("/", "/status", "/guestbook")