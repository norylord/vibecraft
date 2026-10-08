#!/bin/sh
# cargo runner: подписать бинарник dev-сертификатом и запустить.
# Сертификата нет (другая машина, CI) — запускаем без подписи, как обычно
codesign -s "Diogen Dev" -i studio.lince.diogen -f "$1" >/dev/null 2>&1
exec "$@"
