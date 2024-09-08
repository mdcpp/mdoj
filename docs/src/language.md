---
title: "Adding Language"
nav_order: 3
collapse: false
---

Adding programing language to MDOJ very **simple**.

1. Download programing languages plugins from release(`languages.zip`).
2. Extract the compressed files, and pick `lang` file of your choice.
3. Move those `lang` file to `plugin folder.

> `Getting Started` is for demonstration, it's harder to update/backup using it,
> use it at your own risk!

## Where is the `plugin` folder

---

| Case                | location                               |
| ------------------- | -------------------------------------- |
| `Getting Started`   | relative path `./plugins` in folder    |
| `Deployment Docker` | `./judger/plugins` in generated folder |
| `Development Docker | `./judger/plugins` in default folder   |

---

> When running script, MDOJ might emit warning: `unsupported entry type: Link`,
> it should still works, you can ignore that for now.
