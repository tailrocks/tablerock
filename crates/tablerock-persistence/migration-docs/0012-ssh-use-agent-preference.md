# 0012 SSH agent preference

Adds the closed Boolean `ssh_use_agent` profile preference. Existing profiles
default to disabled, preserving prior authentication behavior until an operator
explicitly selects agent use.
