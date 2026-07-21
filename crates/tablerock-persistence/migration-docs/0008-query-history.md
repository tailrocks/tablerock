# 0008 query history

Adds bounded local query history with closed engine/outcome values and indexes
for newest-first and per-engine reads. Statement text is nullable so private or
metadata-only retention never requires SQL persistence. Result payloads and
cell values have no columns.
