# Plan 017 residual — SSH client keepalive defaults

Date: 2026-07-18

## What landed

- `ssh_client_config()` used for every bastion session
- `keepalive_interval` = **30 s**
- `keepalive_max` = **3** unanswered → connection close
- Constants: `SSH_KEEPALIVE_INTERVAL_SECS`, `SSH_KEEPALIVE_MAX`
- Unit: `client_config_enables_keepalive` (asserts override of russh default `None`)

## Why

Idle operator sessions through NAT/bastion idle timers drop the SSH control
channel without application traffic; russh defaults leave keepalive off.

## Commands

```bash
cargo test -p tablerock-engine --lib client_config_enables_keepalive
```
