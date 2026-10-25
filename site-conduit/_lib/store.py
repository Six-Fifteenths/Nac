"""RealWorld（Conduit）的数据层。

原版规格：https://github.com/gothinkster/realworld （82k star）
规格里分 User / Article / Comment / Profile 四块，关系是：用户写文章、
文章带标签、读者收藏文章、读者关注作者、文章下面有评论。

这里全部落在一个 SQLite 文件里（data/appdata/site.db），表结构就是规格里的关系，
不多不少。页面只调这里的函数，不写 SQL。
"""

import hashlib
import re
import time

import nac

SALT = "conduit-nac-2026"

SCHEMA = """
create table if not exists users(
  id integer primary key,
  username text not null unique,
  email text not null unique,
  pass text not null,
  bio text not null default '',
  image text not null default '',
  created text not null
);
create table if not exists followers(
  follower_id integer not null,
  followee_id integer not null,
  primary key (follower_id, followee_id)
);
create table if not exists articles(
  id integer primary key,
  slug text not null unique,
  title text not null,
  description text not null,
  body text not null,
  author_id integer not null,
  created text not null,
  updated text not null
);
create table if not exists article_tags(
  article_id integer not null,
  tag text not null,
  primary key (article_id, tag)
);
create table if not exists favorites(
  user_id integer not null,
  article_id integer not null,
  primary key (user_id, article_id)
);
create table if not exists comments(
  id integer primary key,
  article_id integer not null,
  author_id integer not null,
  body text not null,
  created text not null
);
"""

PAGE_SIZE = 10


def connect():
    db = nac.db()
    db.executescript(SCHEMA)
    return db


def now():
    return time.strftime("%Y-%m-%d %H:%M")


def hash_pw(username, password):
    return hashlib.sha256((SALT + username.lower() + ":" + password).encode("utf-8")).hexdigest()


def slugify(title):
    """标题 → 地址的一段。中文留着：Nac 的路径本来就是 UTF-8。"""
    text = re.sub(r"[\W_]+", "-", title.lower()).strip("-")
    text = re.sub(r"-{2,}", "-", text)[:48].strip("-")
    return text or "article"


# ------------------------------------------------------------------- users


def create_user(db, username, email, password):
    if not username or not email or not password:
        return None, "用户名、邮箱、密码都要填。"
    if db.execute("select 1 from users where username=?", (username,)).fetchone():
        return None, "这个用户名已经有人用了。"
    if db.execute("select 1 from users where email=?", (email,)).fetchone():
        return None, "这个邮箱已经注册过了。"
    db.execute(
        "insert into users(username, email, pass, created) values(?,?,?,?)",
        (username, email, hash_pw(username, password), now()),
    )
    db.commit()
    return db.execute("select * from users where username=?", (username,)).fetchone(), ""


def login(db, email, password):
    row = db.execute("select * from users where email=?", (email,)).fetchone()
    if row is None or row["pass"] != hash_pw(row["username"], password):
        return None
    return row


def user_by_name(db, username):
    return db.execute("select * from users where username=?", (username,)).fetchone()


def update_user(db, uid, email, bio, password):
    row = db.execute("select * from users where id=?", (uid,)).fetchone()
    if row is None:
        return "登录状态过期了。"
    if email and email != row["email"]:
        if db.execute("select 1 from users where email=?", (email,)).fetchone():
            return "这个邮箱已经被别人用了。"
    if password:
        db.execute(
            "update users set email=?, bio=?, pass=? where id=?",
            (email or row["email"], bio, hash_pw(row["username"], password), uid),
        )
    else:
        db.execute(
            "update users set email=?, bio=? where id=?",
            (email or row["email"], bio, uid),
        )
    db.commit()
    return ""


def following(db, follower_id, followee_id):
    if not follower_id or not followee_id:
        return False
    return (
        db.execute(
            "select 1 from followers where follower_id=? and followee_id=?",
            (follower_id, followee_id),
        ).fetchone()
        is not None
    )


def set_follow(db, follower_id, followee_id, on):
    if not follower_id or follower_id == followee_id:
        return
    if on:
        db.execute(
            "insert or ignore into followers(follower_id, followee_id) values(?,?)",
            (follower_id, followee_id),
        )
    else:
        db.execute(
            "delete from followers where follower_id=? and followee_id=?",
            (follower_id, followee_id),
        )
    db.commit()


# ---------------------------------------------------------------- articles

ARTICLE_SQL = """
select a.id, a.slug, a.title, a.description, a.body, a.created, a.updated,
       u.id as author_id, u.username as author, u.bio as author_bio, u.image as author_image,
       (select count(*) from favorites f where f.article_id = a.id) as fav_count,
       (select count(*) from comments c where c.article_id = a.id) as comment_count,
       exists(select 1 from favorites f where f.article_id = a.id and f.user_id = :viewer) as favorited,
       exists(select 1 from followers fo where fo.follower_id = :viewer and fo.followee_id = u.id) as following,
       (select group_concat(tag, ',') from (select tag from article_tags where article_id = a.id order by tag)) as tags
from articles a join users u on u.id = a.author_id
"""


def _rows(db, sql, params):
    return db.execute(sql, params).fetchall()


def article_list(db, viewer_id, tag=None, author=None, favorited_by=None, feed_of=None, page=1):
    sql = ARTICLE_SQL
    params = {"viewer": viewer_id or 0}
    where = []
    if tag:
        where.append("a.id in (select article_id from article_tags where tag = :tag)")
        params["tag"] = tag
    if author:
        where.append("u.username = :author")
        params["author"] = author
    if favorited_by:
        where.append(
            "a.id in (select article_id from favorites f2 join users u2 on u2.id = f2.user_id"
            " where u2.username = :favby)"
        )
        params["favby"] = favorited_by
    if feed_of:
        where.append(
            "a.author_id in (select followee_id from followers where follower_id = :feedof)"
        )
        params["feedof"] = feed_of
    if where:
        sql += " where " + " and ".join(where)
    sql += " order by a.id desc limit :lim offset :off"
    params["lim"] = PAGE_SIZE
    params["off"] = max(0, (page - 1) * PAGE_SIZE)
    return _rows(db, sql, params)


def count_articles(db, tag=None, author=None, favorited_by=None, feed_of=None):
    sql = "select count(*) as n from articles a join users u on u.id = a.author_id"
    params = {}
    where = []
    if tag:
        where.append("a.id in (select article_id from article_tags where tag = :tag)")
        params["tag"] = tag
    if author:
        where.append("u.username = :author")
        params["author"] = author
    if favorited_by:
        where.append(
            "a.id in (select article_id from favorites f2 join users u2 on u2.id = f2.user_id"
            " where u2.username = :favby)"
        )
        params["favby"] = favorited_by
    if feed_of:
        where.append(
            "a.author_id in (select followee_id from followers where follower_id = :feedof)"
        )
        params["feedof"] = feed_of
    if where:
        sql += " where " + " and ".join(where)
    return db.execute(sql, params).fetchone()["n"]


def article_by_slug(db, viewer_id, slug):
    rows = _rows(db, ARTICLE_SQL + " where a.slug = :slug", {"viewer": viewer_id or 0, "slug": slug})
    return rows[0] if rows else None


def tags_of(db, limit=20):
    return [
        row["tag"]
        for row in db.execute(
            "select tag, count(*) as n from article_tags group by tag order by n desc, tag limit ?",
            (limit,),
        )
    ]


def save_article(db, author_id, title, description, body, tags, slug=None):
    if not title or not description or not body:
        return None, "标题、简介、正文都要填。"
    stamp = now()
    # 标签会变成地址的一段，所以只留能进路径的字符。
    tag_list = sorted(
        {re.sub(r"[^a-z0-9+#.-]+", "-", t.strip().lower()).strip("-") for t in tags.replace("，", ",").split(",")}
        - {"", None}
    )
    if slug:
        row = db.execute("select * from articles where slug=?", (slug,)).fetchone()
        if row is None or row["author_id"] != author_id:
            return None, "没有这篇文章，或者不是你写的。"
        db.execute(
            "update articles set title=?, description=?, body=?, updated=? where id=?",
            (title, description, body, stamp, row["id"]),
        )
        article_id = row["id"]
    else:
        base = slugify(title)
        candidate = base
        n = 2
        while db.execute("select 1 from articles where slug=?", (candidate,)).fetchone():
            candidate = "%s-%d" % (base, n)
            n += 1
        cur = db.execute(
            "insert into articles(slug, title, description, body, author_id, created, updated)"
            " values(?,?,?,?,?,?,?)",
            (candidate, title, description, body, author_id, stamp, stamp),
        )
        article_id = cur.lastrowid
        slug = candidate
    db.execute("delete from article_tags where article_id=?", (article_id,))
    for tag in tag_list:
        db.execute("insert or ignore into article_tags(article_id, tag) values(?,?)", (article_id, tag))
    db.commit()
    return slug, ""


def delete_article(db, author_id, slug):
    row = db.execute("select * from articles where slug=?", (slug,)).fetchone()
    if row is None or row["author_id"] != author_id:
        return False
    db.execute("delete from article_tags where article_id=?", (row["id"],))
    db.execute("delete from favorites where article_id=?", (row["id"],))
    db.execute("delete from comments where article_id=?", (row["id"],))
    db.execute("delete from articles where id=?", (row["id"],))
    db.commit()
    return True


def set_favorite(db, user_id, slug, on):
    if not user_id:
        return
    row = db.execute("select id from articles where slug=?", (slug,)).fetchone()
    if row is None:
        return
    if on:
        db.execute(
            "insert or ignore into favorites(user_id, article_id) values(?,?)",
            (user_id, row["id"]),
        )
    else:
        db.execute(
            "delete from favorites where user_id=? and article_id=?", (user_id, row["id"])
        )
    db.commit()


# ---------------------------------------------------------------- comments


def comments_of(db, article_id):
    return db.execute(
        "select c.id, c.body, c.created, u.username as author, u.id as author_id"
        " from comments c join users u on u.id = c.author_id"
        " where c.article_id=? order by c.id desc",
        (article_id,),
    ).fetchall()


def add_comment(db, article_id, author_id, body):
    if not author_id or not body.strip():
        return
    db.execute(
        "insert into comments(article_id, author_id, body, created) values(?,?,?,?)",
        (article_id, author_id, body.strip(), now()),
    )
    db.commit()


def delete_comment(db, comment_id, user_id):
    row = db.execute("select * from comments where id=?", (comment_id,)).fetchone()
    if row is None or row["author_id"] != user_id:
        return
    db.execute("delete from comments where id=?", (comment_id,))
    db.commit()
