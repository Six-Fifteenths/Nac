"""小店的共享逻辑：商品表、购物车、订单。

和主站没有任何关系 —— 它有自己的数据目录、自己的进程、自己的端口。
两个站唯一的共同点是它们都由 Nac 提供，并且共用一个站点注册表。
"""

import time

import nac

SCHEMA = """
create table if not exists product(
    id integer primary key,
    name text not null,
    cents integer not null,
    blurb text not null default '',
    stock integer not null default 0
);
create table if not exists orders(
    id integer primary key autoincrement,
    at integer not null,
    total integer not null,
    note text not null default ''
);
create table if not exists lines(
    order_id integer not null,
    product integer not null,
    qty integer not null,
    cents integer not null
);
"""

SEED = [
    (1, "黄铜小台灯", 14800, "暖光，旋钮调亮度。灯罩是拉丝黄铜，开久了会有一层自己的颜色。", 4),
    (2, "铸铁平底锅", 8900, "26 厘米，无涂层。用得越久越不粘，也越重。", 12),
    (3, "手摇磨豆机", 23500, "圆锥磨芯，粗细可调。空转半分钟就能磨一杯的量。", 7),
    (4, "亚麻围裙", 5600, "厚织亚麻，两个口袋，一个放笔一个放手机。", 20),
    (5, "玻璃储物罐", 3200, "一升装，木盖带硅胶圈。密封到能装咖啡豆。", 30),
    (6, "陶土花瓶", 11900, "手工拉坯，每一只的弧度都不一样。", 5),
]


def connect():
    db = nac.db("shop.db")
    db.executescript(SCHEMA)
    if db.execute("select count(*) from product").fetchone()[0] == 0:
        db.executemany(
            "insert into product(id, name, cents, blurb, stock) values(?, ?, ?, ?, ?)",
            SEED,
        )
        db.commit()
        print("shop: seeded %d products" % len(SEED))
    return db


def money(cents):
    return "¥%.2f" % (cents / 100.0)


def products(db):
    return db.execute("select * from product order by id").fetchall()


def product(db, ident):
    return db.execute("select * from product where id = ?", (ident,)).fetchone()


def cart():
    """会话里的购物车：{商品号: 数量}。"""
    return {int(k): int(v) for k, v in (nac.session_get("cart") or {}).items()}


def save_cart(items):
    nac.session_set("cart", {str(k): v for k, v in items.items() if v > 0})


def add(ident, qty=1):
    items = cart()
    items[ident] = items.get(ident, 0) + qty
    save_cart(items)
    return items


def remove(ident):
    items = cart()
    items.pop(ident, None)
    save_cart(items)
    return items


def cart_lines(db):
    """购物车明细：[(商品行, 数量, 小计分)]，下架的商品直接忽略。"""
    out = []
    for ident, qty in sorted(cart().items()):
        row = product(db, ident)
        if row is None:
            continue
        out.append((row, qty, row["cents"] * qty))
    return out


def cart_count(db):
    return sum(qty for _, qty, _ in cart_lines(db))


def cart_total(db):
    return sum(sub for _, _, sub in cart_lines(db))


def place_order(db, note=""):
    """把购物车写成一张订单，返回订单号；空车返回 None。"""
    lines = cart_lines(db)
    if not lines:
        return None
    total = sum(sub for _, _, sub in lines)
    cursor = db.execute(
        "insert into orders(at, total, note) values(?, ?, ?)",
        (int(time.time()), total, note[:80]),
    )
    order_id = cursor.lastrowid
    for row, qty, sub in lines:
        db.execute(
            "insert into lines(order_id, product, qty, cents) values(?, ?, ?, ?)",
            (order_id, row["id"], qty, sub),
        )
        db.execute(
            "update product set stock = max(0, stock - ?) where id = ?",
            (qty, row["id"]),
        )
    db.commit()
    save_cart({})
    return order_id


def order(db, order_id):
    head = db.execute("select * from orders where id = ?", (order_id,)).fetchone()
    if head is None:
        return None, []
    rows = db.execute(
        "select l.qty, l.cents, p.name from lines l"
        " left join product p on p.id = l.product"
        " where l.order_id = ? order by l.rowid",
        (order_id,),
    ).fetchall()
    return head, rows
