# nac:title flappy.bird — 结束
# nac:path /over

"""这一局结束了。看一眼撞在哪，然后决定要不要留个名字。"""

import nac

from _lib import game


def sid():
    return nac.session_get("sid")


game.ensure_tables(nac)
me = sid()
run = game.load(nac, me) if me else None

if run is None or not run["over"]:
    nac.redirect_path("/play")

best = game.best_of(nac, me)
reason = run.get("reason") or "结束了"

nac.h1("撞了：%s" % reason)
with nac.cols(3):
    nac.p(nac.b("分数"), " ", str(run["score"]))
    nac.col()
    nac.p(nac.b("格数"), " ", str(run["tick"]))
    nac.col()
    nac.p(nac.b("最好"), " ", str(best))
nac.gap()

nac.write("\n".join(game.frame(run)))
nac.gap()

nac.h2("留个名字")
nac.p(nac.i("写上去就进排行榜；不写也行，直接重开。"))
nac.form_path("/score")
nac.field("name", "你的名字")
nac.button("记上榜")
nac.gap()

with nac.cols(2):
    nac.p("　", nac.inline_path("/restart", "再来一局"))
    nac.col()
    nac.p("　", nac.inline_path("/", "回首页看排行榜"))
