# 0011 SSH properties and startup actions

Transactionally rebuilds saved profile properties to admit the six approved SSH
property ordinals while retaining every existing row, constraint, and secret-
source representation. Adds at most 16 bounded startup actions per profile with
closed safety class, timeout, and reconnect policy. Profile deletion cascades
through both tables.
