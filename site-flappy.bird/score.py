# nac:title flappy.bird — 记榜
# nac:path /score

"""收下名字和分数，写进排行榜，然后回首页。"""

import nac

from _lib import game


game.ensure_tables(nac)
me = nac.session_get("sid")
run = game.load(nac, me) if me else None

if run and nac.req.submitted():
    game.submit_score(nac, nac.req.field_of("name"), run["score"])

nac.redirect_path("/")
