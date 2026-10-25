# nac:title flappy.bird — 玩
# nac:path /play

"""一帧一请求的游戏页。

每次提交都是一次拍翅：服务器把这一局往前推一格，然后把新的画面发回来。
整个游戏没有循环、没有定时器、没有客户端脚本 —— 只有「点一下，算一步」。
"""

import random

import nac

from _lib import game


def sid():
    """没有就当场建一个 —— 不跳转。

    服务端只跟一次重定向（第二次跳转会被当成环），所以"没开局就跳去 /restart"
    这条路走不通：那会变成 play → restart → play 两次跳转。就地开一局更省事。
    """
    value = nac.session_get("sid")
    if not value:
        value = "%08x" % random.getrandbits(32)
        nac.session_set("sid", value)
    return value


game.ensure_tables(nac)
me = sid()

# 第一次来：就地开一局，不跳转
run = game.load(nac, me)
if run is None:
    run = game.new_run()
    game.save(nac, me, run, game.best_of(nac, me))

if run["over"]:
    nac.redirect_path("/over")

# ---------------------------------------------------------------- 这就是一帧
# 隐藏字段里带着「这一帧是第几格」：一来让 submitted() 能认出这是提交
# （空表单的 fields 是空的），二来挡住连点两下把一步走成两步。
if nac.req.submitted() and nac.req.field_of("t") == str(run["tick"]):
    game.flap(run)
    best = max(game.best_of(nac, me), run["score"])
    game.save(nac, me, run, best)
    if run["over"]:
        nac.redirect_path("/over")
    else:
        nac.redirect_path("/play")

best = game.best_of(nac, me)

nac.write("\n".join(game.frame(run)))

with nac.cols(3):
    nac.p(nac.b("分数"), " ", str(run["score"]))
    nac.col()
    nac.p(nac.b("第几格"), " ", str(run["tick"]))
    nac.col()
    nac.p(nac.b("最好"), " ", str(best))
nac.gap()

nac.form_path("/play")
nac.hidden("t", str(run["tick"]))
nac.button("拍一下")
nac.gap()

with nac.cols(2):
    nac.p("　", nac.inline_path("/restart", "重开一局"))
    nac.col()
    nac.p("　", nac.inline_path("/", "回首页"))

nac.gap()
nac.p(nac.i("点一次按钮 = 鸟飞一下 + 世界走一格。想停下来就停，这局会一直等着你。"))
