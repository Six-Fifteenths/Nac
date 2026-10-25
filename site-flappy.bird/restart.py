# nac:title flappy.bird — 新的一局
# nac:path /restart

"""重置这一局，然后跳回 /play。单独一页，因为「一页只能有一个表单」。"""

import nac

from _lib import game


def sid():
    value = nac.session_get("sid")
    if not value:
        import random

        value = "%08x" % random.getrandbits(32)
        nac.session_set("sid", value)
    return value


game.ensure_tables(nac)
me = sid()
game.save(nac, me, game.new_run(), game.best_of(nac, me))
nac.redirect_path("/play")
