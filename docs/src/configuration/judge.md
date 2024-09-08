---
title: "Judge"
parent: "Configuration"
nav_order: 1
---

## What's judge

Judge is an sandboxing program that can execute user-submitted code securely
without running a virtual machine.

## How to change config?

You need to find a toml file and change its content.

File can be found in following path:

---

| Case                | location                                          |
| ------------------- | ------------------------------------------------- |
| `Getting Started`   | relative path `./judger.toml` in folder           |
| `Deployment Docker` | `./judger/config/config.toml` in generated folder |
| `Development Docker | `./judger/config/config.toml` in default folder   |

---

## Default configuration

```toml
to_be_filled = true
```

## Common change

- usable memory
