"""Flappy Bird 的全部规则。

这个游戏没有帧循环 —— 一帧就是一次请求，一次点击就是一次「拍翅 + 走一步」。
所以物理是离散的：每次提交推进一个 tick，画出来的就是一帧静止画面。

坐标系：世界宽 540、高 300（场景单位），鸟固定在 x=90，水管从右往左走。
"""

import json
import random

# ---- 可调的几个数（改完重新发布就能看到效果）
TICK_GRAVITY = 3.4
TICK_FLAP = -9.0
TICK_SPEED = 9.0
PIPE_WIDTH = 54
PIPE_GAP = 104
PIPE_SPACING = 210
SPAWN_AT = 400          # 最后一根水管走到这个 x 之前，就来新的一根
BIRD_X = 90
BIRD_W = 24
BIRD_H = 18
FIELD_W = 540
FIELD_H = 300
GROUND_Y = 262          # 地面从这里开始，碰地就算输
CEILING = 0


def new_run(seed=None):
    """新的一局。seed 决定水管的随机排布，存进去就能复盘同一局。"""
    seed = seed if seed is not None else random.randint(1, 10 ** 9)
    rng = random.Random(seed)
    return {
        "seed": seed,
        "tick": 0,
        "y": 150.0,
        "vy": 0.0,
        "score": 0,
        "pipes": [
            {"x": 420.0, "gap": _gap(rng), "passed": False},
            {"x": 420.0 + PIPE_SPACING, "gap": _gap(rng), "passed": False},
        ],
        "over": False,
        "reason": "",
    }


def _gap(rng):
    """缺口的上沿：上下各留 40 的余量，免得水管贴边没法过。"""
    return rng.uniform(40.0, GROUND_Y - PIPE_GAP - 40.0)


def flap(run):
    """拍一下：给一个向上的速度，然后走一步。"""
    if run["over"]:
        return run
    run["vy"] = TICK_FLAP
    return step(run)


def step(run):
    """走一步。整局游戏只有这里推进时间。"""
    if run["over"]:
        return run

    run["tick"] += 1
    run["vy"] += TICK_GRAVITY
    run["y"] += run["vy"]

    for pipe in run["pipes"]:
        pipe["x"] -= TICK_SPEED

    if run["pipes"] and run["pipes"][-1]["x"] <= SPAWN_AT:
        rng = random.Random(run["seed"] * 7919 + run["tick"])
        run["pipes"].append(
            {
                "x": run["pipes"][-1]["x"] + PIPE_SPACING,
                "gap": _gap(rng),
                "passed": False,
            }
        )
    run["pipes"] = [p for p in run["pipes"] if p["x"] > -PIPE_WIDTH - 10]

    for pipe in run["pipes"]:
        if not pipe["passed"] and pipe["x"] + PIPE_WIDTH < BIRD_X:
            pipe["passed"] = True
            run["score"] += 1

    run["reason"] = _hit(run)
    if run["reason"]:
        run["over"] = True
    return run


def _hit(run):
    top = run["y"]
    bottom = run["y"] + BIRD_H
    if top < CEILING:
        return "撞到天花板"
    if bottom > GROUND_Y:
        return "落地了"
    for pipe in run["pipes"]:
        if pipe["x"] < BIRD_X + BIRD_W and pipe["x"] + PIPE_WIDTH > BIRD_X:
            if top < pipe["gap"] or bottom > pipe["gap"] + PIPE_GAP:
                return "撞到水管"
    return ""


# ----------------------------------------------------------------- 画一帧
#
# 配色参考了经典作品的用色关系（天空蓝 / 水管绿 / 土地黄）。颜色本身不受版权
# 保护，形状也是自己拼的：这个平台只画得出圆角矩形，所以鸟和水管都是矩形拼的。
#
# 两个技巧：
# - 天空、地面、山、云都画得比画布大，四角落在画布外侧，客户端裁剪会把圆角吃掉。
# - 水管缺口那一端本来就该是圆的，圆角矩形刚好当「管口」；再叠一块更宽更矮的
#   当「管唇」，就接近原版的形状了。

SKY = "70c5ce"
CLOUD = "e8f7fb"
PIPE = "73bf2e"
PIPE_DARK = "5b9c22"
PIPE_LIGHT = "a8e05f"
GROUND = "ded895"
GROUND_LINE = "c9c078"
GRASS = "9ade4a"
BIRD = "f7d51d"
BIRD_WING = "e8c51a"
BEAK = "f5a623"
EYE = "3b3b3b"


def background():
    """静止的背景：天空、云、地面。全部超出画布，圆角落在外面被裁掉。"""
    return [
        (SKY, 700, 460, -80, -80),
        (CLOUD, 96, 18, 70, 44),
        (CLOUD, 58, 14, 150, 38),
        (CLOUD, 80, 16, 330, 70),
        (CLOUD, 44, 12, 400, 62),
        (GROUND, 700, 96, -80, GROUND_Y - 6),
        (GRASS, 700, 6, -80, GROUND_Y - 6),
        (GROUND_LINE, 700, 4, -80, GROUND_Y + 14),
    ]


def pipe_parts(gap):
    """一根水管的所有零件，坐标相对「水管左上角」。

    ((颜色, 宽, 高, dx, dy), …) —— 调用方决定这根水管在哪。
    """
    top_h = max(0.0, gap) + 20
    bottom_y = gap + PIPE_GAP
    bottom_h = max(20.0, FIELD_H - bottom_y + 20)
    cap_top = gap - 14
    parts = [
        (PIPE, PIPE_WIDTH, top_h, 0, -20),
        (PIPE_LIGHT, 8, top_h, 8, -20),
        (PIPE, PIPE_WIDTH, bottom_h, 0, bottom_y),
        (PIPE_LIGHT, 8, bottom_h, 8, bottom_y),
        (PIPE_DARK, PIPE_WIDTH + 10, 14, -5, bottom_y),
    ]
    if cap_top > 0:
        parts.append((PIPE_DARK, PIPE_WIDTH + 10, 14, -5, cap_top))
    return parts


def bird_parts(y, tick=0):
    """鸟的零件，坐标相对「鸟的左上角」。翅膀每两格动一下。"""
    wing = 0 if (tick // 2) % 2 == 0 else 4
    return [
        (BIRD, BIRD_W, BIRD_H, 0, 0),
        (BIRD_WING, 10, 6, 4, 8 + wing),
        (BEAK, 9, 6, BIRD_W - 2, 7),
        (EYE, 4, 4, 14, 4),
    ]


def frame(run):
    """把这一局画成一个 `[[scene]]`，返回若干行标记。"""
    bars = list(background())
    for pipe in run["pipes"]:
        for color, w, h, dx, dy in pipe_parts(pipe["gap"]):
            bars.append((color, w, h, pipe["x"] + dx, dy))
    for color, w, h, dx, dy in bird_parts(run["y"], run["tick"]):
        bars.append((color, w, h, BIRD_X + dx, run["y"] + dy))

    out = ["[[scene %dx%d 0.01]]" % (FIELD_W, FIELD_H)]
    for color, w, h, x, y in bars:
        out.append("[[bar %s %d %d]]" % (color, round(w), round(h)))
        out.append("[[at 0 %.1f %.1f]]" % (x, y))
    out.append("[[end]]")
    return out


# ----------------------------------------------------------------- 存一局

def ensure_tables(nac):
    db = nac.db()
    db.execute(
        "create table if not exists runs "
        "(sid text primary key, state text, best integer default 0)"
    )
    db.execute("create table if not exists scores (name text, best integer, at text)")
    db.commit()


def load(nac, sid):
    db = nac.db()
    row = db.execute("select state, best from runs where sid = ?", (sid,)).fetchone()
    if not row or not row["state"]:
        return None
    try:
        return json.loads(row["state"])
    except (ValueError, TypeError):
        return None


def save(nac, sid, run, best):
    db = nac.db()
    db.execute(
        "insert into runs (sid, state, best) values (?, ?, ?) "
        "on conflict(sid) do update set state = excluded.state, "
        "best = max(runs.best, excluded.best)",
        (sid, json.dumps(run), int(best)),
    )
    db.commit()


def best_of(nac, sid):
    db = nac.db()
    row = db.execute("select best from runs where sid = ?", (sid,)).fetchone()
    return int(row["best"]) if row else 0


def leaderboard(nac, limit=8):
    db = nac.db()
    return db.execute(
        "select name, best from scores order by best desc, at asc limit ?", (limit,)
    ).fetchall()


def submit_score(nac, name, score):
    name = (name or "").strip()[:24] or "匿名鸟"
    db = nac.db()
    db.execute(
        "insert into scores (name, best, at) values (?, ?, datetime('now'))",
        (name, int(score)),
    )
    db.commit()
