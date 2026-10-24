# nac:title 运行状态
# nac:path /status

import json
import time

import nac

views = nac.counter("status-views.json")
guest_count = len(nac.read_lines("guestbook.txt"))
name = nac.session_get("name")
mood = nac.session_get("mood")

gen = "?"
pages = []
try:
    data_root = nac.data_dir().parent
    gen = (data_root / "current").read_text(encoding="utf-8").strip()
    manifest = json.loads((data_root / "gen" / gen / "manifest.json").read_text(encoding="utf-8"))
    pages = manifest.get("pages", [])
except Exception:
    pass

static = sum(1 for page in pages if page.get("kind") == "static")
dynamic = sum(1 for page in pages if page.get("kind") == "dynamic")

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
    nac.p("页面 ", nac.b(len(pages)), " 个（静态 ", static, " / 动态 ", dynamic, "）。")
    nac.p("图片 ", nac.b(len(nac.IMAGES)), " 张。")

nac.gap()
nac.h2("站点数据")
nac.bullets(
    "留言板累计 %d 条。" % guest_count,
    "状态页计数、留言和会话都写在 data/appdata/ 下。",
    "写文件使用临时文件 + 原子替换，不会留下半截 JSON。",
)

nac.gap()
nac.h2("你的会话")
if name or mood:
    nac.p("名字：", nac.b(name or "（空）"))
    nac.p("心情：", nac.i(mood or "（空）"))
else:
    nac.p("还没有会话数据。去 ", nac.tt("/interactive"), " 提交一次表单，再回来看看。")

nac.gap()
nac.h2("观察点")
nac.bullets(
    "静态页的访问不会出现在 Python 启动记录里；动态页会。",
    "内容更新只需要重新发布，不用重启 contentd。",
    "页面失败统一返回空响应，原因只写服务端日志。",
)
nac.gap()
nac.link_path("/interactive", "互动实验")
nac.link_path("/guestbook", "留言板")
nac.link_path("/", "回首页")
