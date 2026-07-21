# 0016 history retention

Adds the singleton history-retention preference with three closed policy values
and initializes existing databases to full retention. Runtime writes still
apply the selected policy before persistence, so private/metadata modes do not
depend on later redaction.
