# 0010 column layout

Adds one bounded JSON column-layout document per profile/database/schema/table.
The document records presentation state such as visibility, order, and width;
it never stores cell values. The composite primary key keeps engine context
explicit.
