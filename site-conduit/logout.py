# nac:title 退出
# nac:path /logout

"""退出登录：清掉会话，回首页。没有表单，纯动作。"""

import nac

from _lib import auth

auth.sign_out()
nac.redirect_path("/")
