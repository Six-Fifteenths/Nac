"""汉诺塔：移动序列、几何布局、时间轴。

这个模块只算东西，不碰 nac 的排版函数 —— 它返回标记行，页面负责塞进去。
"""

#: 画布。宽度固定，高度按层数算 —— 层数少的时候不该留一大片空白。
CANVAS_W = 640
PEG_X = (110, 320, 530)

#: 盘子。
DISK_H = 16
MIN_W = 52
MAX_W = 176
GAP = 2

#: 抬起来的那一行：盘子的上沿离画布顶这么远。
LIFT_Y = 6

#: 画布底部留白。
FLOOR = 22


def height(n):
    """画布高度。抬起的那一行 + 柱顶余量 + 盘子摞 + 底座 + 底部留白。"""
    return base_y(n) + FLOOR


def peg_top(n):
    """柱子顶端。比抬起的盘子低 8 像素，所以飞过去的盘子不会压在柱头上。"""
    return LIFT_Y + DISK_H + 8


def base_y(n):
    """底座上沿。柱子刚好高过最高的那一摞 18 像素。"""
    return peg_top(n) + n * (DISK_H + GAP) + 18

#: 每个盘子的颜色，大的先，所以相邻不会撞色。
PALETTE = (
    "c0392b",
    "d35400",
    "c9a227",
    "27865b",
    "1f6f8b",
    "5b3d8c",
    "a33a5b",
    "4a5568",
)


def solve(n):
    """经典递归解。返回 [(从, 到), ...]，柱子用 0/1/2 表示。

    这就是整个演示的全部「逻辑」：2^n - 1 步，n=20 时超过一百万步。
    """
    moves = []

    def walk(count, src, dst, spare):
        if count == 0:
            return
        walk(count - 1, src, spare, dst)
        moves.append((src, dst))
        walk(count - 1, spare, dst, src)

    walk(n, 0, 2, 1)
    return moves


def disk_width(index, n):
    """第 `index` 个盘子（0 是最大的）的宽度。"""
    if n <= 1:
        return MAX_W
    return MAX_W - (MAX_W - MIN_W) * index / (n - 1)


def _disk_x(index, n, peg):
    return PEG_X[peg] - disk_width(index, n) / 2


def _stack_y(level, n):
    """一摞盘子从下往上第 `level` 层（0 是贴底那一层）的上沿。"""
    return base_y(n) - (level + 1) * (DISK_H + GAP)


def per_move(n, speed=1.0):
    """一步用多少秒。盘子越多整体越快，否则 7 层要看两分钟。"""
    if n <= 3:
        base = 1.05
    elif n <= 5:
        base = 0.58
    elif n == 6:
        base = 0.38
    else:
        base = 0.28
    return base / max(0.25, speed)


def timeline(n, speed=1.0):
    """把移动序列翻成时间轴，返回 (标记行, 总秒数, 步数)。

    一步拆成四个关键帧（原位 → 抬起 → 平移到目标上方 → 落下），客户端在关键帧
    之间做平滑插值，所以看起来是抬手、滑过去、放下，而不是瞬移。

    每个盘子只声明一次，关键帧都挂在它自己名下 —— 声明两次会让后一个从第 0 秒
    起就画在终点上。
    """
    moves = solve(n)
    step_seconds = per_move(n, speed)
    floor_y = base_y(n)
    top = peg_top(n)
    lift = LIFT_Y

    heights = [n, 0, 0]
    where = {index: 0 for index in range(n)}
    keys = {
        index: [(0.0, _disk_x(index, n, 0), _stack_y(index, n))]
        for index in range(n)
    }

    t = 0.0
    for src, dst in moves:
        # 每次移动的永远是源柱顶上那个盘子。
        disk = next(i for i in range(n - 1, -1, -1) if where[i] == src)
        level = heights[src] - 1
        start_x = _disk_x(disk, n, src)
        end_x = _disk_x(disk, n, dst)
        from_y = _stack_y(level, n)
        to_y = _stack_y(heights[dst], n)

        heights[src] -= 1
        heights[dst] += 1
        where[disk] = dst

        keys[disk].append((t, start_x, from_y))
        keys[disk].append((t + step_seconds * 0.32, start_x, lift))
        keys[disk].append((t + step_seconds * 0.74, end_x, lift))
        keys[disk].append((t + step_seconds, end_x, to_y))
        t += step_seconds

    lines = ["[[scene %dx%d %.2f]]" % (CANVAS_W, height(n), t)]

    # 柱子和底座先画，盘子在它们上面。
    for peg in range(3):
        lines.append("[[bar 6b7280 8 %d]]" % (floor_y - top))
        lines.append("[[at 0 %.0f %d]]" % (PEG_X[peg] - 4, top))
    lines.append("[[bar a7aeb8 %d 10]]" % (CANVAS_W - 40))
    lines.append("[[at 0 20 %d]]" % (floor_y - 10))

    for index in range(n):
        lines.append(
            "[[bar %s %.0f %.0f]]"
            % (PALETTE[index % len(PALETTE)], disk_width(index, n), DISK_H)
        )
        for at, x, y in keys[index]:
            # Whole pixels: the client interpolates in floats, so keeping the
            # keyframes coarse buys a few KB and costs nothing visible.
            lines.append("[[at %.2f %.0f %.0f]]" % (at, x, y))

    lines.append("[[end]]")
    return lines, t, len(moves)
