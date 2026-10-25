"""Hacker News 复刻的共享逻辑：排名、时间、登录、评论树。

这是一个普通的 Python 模块，被页面 `import` 进来用。它不拼页面，只管数据和
计算 —— 排版留在各个页面里，这样换一个页面样式不用动这里。

排名公式是 HN 自己用的那一条：

    score = (points - 1) / (age_hours + 2) ** 1.8

新条目有一条基础的 1 分，所以 `points - 1` 只数别人给的票。
"""

import hashlib
import json
import os
import secrets
import time

import nac

#: HN 的重力指数。调大 = 旧帖子掉得更快。
GRAVITY = 1.8

#: 一页多少条。
PER_PAGE = 15

#: 评论树最多缩进几层。再深的回复继续平铺，免得窄窗口里挤成一列字。
MAX_DEPTH = 4

#: 一次最多渲染多少条评论，以及单条正文截到多少字。
MAX_COMMENTS = 25
MAX_BODY = 400

STORY = 1
COMMENT = 2

_SCHEMA = """
create table if not exists item(
    id integer primary key autoincrement,
    kind integer not null,
    parent integer not null default 0,
    title text not null default '',
    url text not null default '',
    body text not null default '',
    author text not null default '',
    at integer not null,
    dead integer not null default 0
);
create table if not exists vote(
    user text not null,
    item integer not null,
    primary key(user, item)
);
create table if not exists account(
    name text primary key,
    salt text not null,
    pw text not null,
    about text not null default '',
    created integer not null
);
create index if not exists item_parent on item(parent);
create index if not exists item_at on item(at);
create index if not exists item_kind on item(kind);
"""


# ------------------------------------------------------------------ 数据库

def connect():
    """打开站点数据库，建表，必要时灌一次种子数据。"""
    db = nac.db("hn.db")
    db.executescript(_SCHEMA)
    _seed(db)
    return db


def _seed_path():
    here = os.path.dirname(os.path.abspath(__file__))
    return os.path.join(os.path.dirname(here), "_data", "hn_seed.json")


def _seed(db):
    """第一次运行时把种子故事、票数和评论写进库；之后库里有东西就不管了。"""
    have = db.execute("select count(*) from item").fetchone()[0]
    if have:
        return
    try:
        with open(_seed_path(), encoding="utf-8") as handle:
            data = json.load(handle)
    except OSError:
        return
    now = int(time.time())
    for index, story in enumerate(data.get("stories", [])):
        at = now - int(story.get("age", 3600))
        cursor = db.execute(
            "insert into item(kind, parent, title, url, body, author, at)"
            " values(?, 0, ?, ?, ?, ?, ?)",
            (
                STORY,
                story.get("title", ""),
                story.get("url", ""),
                story.get("text", ""),
                story.get("by", "anon"),
                at,
            ),
        )
        story_id = cursor.lastrowid
        # 种子票数用一批虚构用户投，这样排名一开始就有区分度。
        for n in range(int(story.get("votes", 0))):
            db.execute(
                "insert or ignore into vote(user, item) values(?, ?)",
                ("voter%d_%d" % (index, n), story_id),
            )
        _seed_comments(db, story_id, story.get("comments") or [], now)
    db.commit()
    print("hn: seeded %d stories" % len(data.get("stories", [])))


def _seed_comments(db, parent, items, now):
    for entry in items:
        at = now - int(entry.get("age", 0))
        cursor = db.execute(
            "insert into item(kind, parent, body, author, at) values(?, ?, ?, ?, ?)",
            (COMMENT, parent, entry.get("text", ""), entry.get("by", "anon"), at),
        )
        _seed_comments(db, cursor.lastrowid, entry.get("replies") or [], now)


# -------------------------------------------------------------------- 排名

def score(points, at, now=None):
    """HN 的排名分。越大越靠前。"""
    now = time.time() if now is None else now
    hours = max(0.0, (now - at) / 3600.0)
    return (points - 1.0) / ((hours + 2.0) ** GRAVITY)


def _story_rows(db, where="", args=()):
    """带票数与回复数的一次性查询，避免每条再单独查两遍。"""
    sql = (
        "select i.*,"
        " (select count(*) from vote v where v.item = i.id) as votes,"
        " (select count(*) from item c where c.parent = i.id and c.dead = 0) as replies"
        " from item i where i.kind = 1 and i.dead = 0 " + where
    )
    return db.execute(sql, args).fetchall()


def ranked(db, page=1, now=None):
    """按分数排好的第 `page` 页，从 1 开始。"""
    now = time.time() if now is None else now
    rows = _story_rows(db)
    rows = sorted(
        rows,
        key=lambda r: score(1 + r["votes"], r["at"], now),
        reverse=True,
    )
    start = max(0, (page - 1) * PER_PAGE)
    return rows[start:start + PER_PAGE]


def newest(db, page=1):
    """按时间排的第 `page` 页。"""
    start = max(0, (page - 1) * PER_PAGE)
    return _story_rows(db, "order by i.at desc limit ? offset ?", (PER_PAGE, start))


def newest_count(db):
    return db.execute("select count(*) from item where kind = 1").fetchone()[0]


def page_count(db):
    return max(1, (newest_count(db) + PER_PAGE - 1) // PER_PAGE)


# -------------------------------------------------------------------- 条目

def item(db, item_id):
    return db.execute("select * from item where id = ?", (item_id,)).fetchone()


def points(db, item_id):
    row = db.execute("select count(*) from vote where item = ?", (item_id,)).fetchone()
    return 1 + row[0]


def voted(db, user, item_id):
    if not user:
        return False
    row = db.execute(
        "select 1 from vote where user = ? and item = ?", (user, item_id)
    ).fetchone()
    return row is not None


def karma(db, name):
    """这个人的总分：他发的东西拿到的票数之和。"""
    row = db.execute(
        "select count(*) from vote v join item i on i.id = v.item where i.author = ?",
        (name,),
    ).fetchone()
    return row[0]


def comments_by(db, name, limit=10):
    return db.execute(
        "select c.*, s.title as story_title from item c"
        " join item s on s.id = c.parent"
        " where c.kind = 2 and c.author = ? and c.dead = 0"
        " order by c.at desc limit ?",
        (name, limit),
    ).fetchall()


def stories_by(db, name, limit=10):
    return _story_rows(db, "and i.author = ? order by i.at desc limit ?", (name, limit))


# ------------------------------------------------------------------ 评论树

def thread(db, story_id):
    """这一条故事下的整棵评论树，返回 (子节点表, 评论总数)。

    用 SQLite 的递归查询把子树一次捞出来 —— 不认识的评论不会混进来，也不用
    把全站的评论都读一遍再过滤。
    """
    rows = db.execute(
        "with recursive sub(id) as ("
        "  select id from item where parent = ? and kind = 2 and dead = 0"
        "  union all"
        "  select i.id from item i join sub on i.parent = sub.id"
        "  where i.kind = 2 and i.dead = 0"
        ") select i.* from item i join sub on sub.id = i.id order by i.at",
        (story_id,),
    ).fetchall()
    kids = {}
    for row in rows:
        kids.setdefault(row["parent"], []).append(row)
    return kids, len(rows)


def root_of(db, comment_id):
    """一条评论所属的故事编号。顺着 parent 一路往上走，带环保护。"""
    seen = set()
    current = comment_id
    while current and current not in seen:
        seen.add(current)
        row = db.execute("select parent, kind from item where id = ?", (current,)).fetchone()
        if row is None:
            return 0
        if row["kind"] == STORY:
            return current
        current = row["parent"]
    return 0


# -------------------------------------------------------------------- 时间

def ago(at, now=None):
    """相对时间，像 HN 那样只说到粗粒度。"""
    now = time.time() if now is None else now
    delta = max(0, int(now - at))
    if delta < 60:
        return "%d 秒前" % delta
    if delta < 3600:
        return "%d 分钟前" % (delta // 60)
    if delta < 86400:
        return "%d 小时前" % (delta // 3600)
    if delta < 86400 * 30:
        return "%d 天前" % (delta // 86400)
    if delta < 86400 * 365:
        return "%d 个月前" % (delta // (86400 * 30))
    return "%d 年前" % (delta // (86400 * 365))


def domain(url):
    """链接的站点名，`https://` 和 `www.` 去掉。"""
    rest = url.split("://", 1)[-1]
    host = rest.split("/", 1)[0]
    return host[4:] if host.startswith("www.") else host


# -------------------------------------------------------------------- 登录

def hash_pw(salt, pw):
    return hashlib.sha256((salt + pw).encode("utf-8")).hexdigest()


def current_user():
    """会话里的用户名，没登录就是空字符串。"""
    return nac.session_get("user") or ""


def sign_in(user):
    nac.session_set("user", user)


def sign_out():
    nac.session_pop("user")


def account(db, name):
    return db.execute("select * from account where name = ?", (name,)).fetchone()


def register(db, name, pw):
    """建号。名字已被占用返回 False。"""
    if not name or not pw:
        return False
    if account(db, name) is not None:
        return False
    salt = secrets.token_hex(8)
    db.execute(
        "insert into account(name, salt, pw, created) values(?, ?, ?, ?)",
        (name, salt, hash_pw(salt, pw), int(time.time())),
    )
    db.commit()
    return True


def authenticate(db, name, pw):
    row = account(db, name)
    if row is None:
        return False
    return secrets.compare_digest(row["pw"], hash_pw(row["salt"], pw))


# ------------------------------------------------------------------ 小工具

def clip(text, limit=MAX_BODY):
    """截断长正文，并标出来。"""
    text = (text or "").strip()
    if len(text) <= limit:
        return text
    return text[:limit].rstrip() + "…"


def pagination(page, total):
    """上一页 / 下一页的路径，到头了就是 None。"""
    prev_path = None if page <= 1 else ("/hn/" if page == 2 else "/hn/page/%d" % (page - 1))
    next_path = None if page >= total else "/hn/page/%d" % (page + 1)
    return prev_path, next_path
