# spineAssets

Spine 角色资源目录。每个角色一个子目录，便于后续继续添加角色。

```
spineAssets/
└── characters/
    └── character_01/          # 第 1 个角色
        ├── skeleton.atlas     # atlas 描述文件（贴图名需与同目录文件一致）
        ├── skeleton.skel.bytes# 二进制骨骼（Spine 4.2.43）
        ├── skeleton.png       # atlas 第 1 页
        └── skeleton_2.png ... # atlas 第 2..N 页
```

## 约定

- 目录名建议使用 `character_01`、`character_02` 这类稳定编号；显示名和实际配置在
  `src/character.rs` 的 `CHARACTERS` 中维护。
- 每个角色使用统一的 `skeleton.*` 命名，避免直接把导出文件名（如 `Character_full`）
  带进代码。
- 当前角色的 atlas 原贴图名为 `Character_full.png` / `Character_full_2.png` ...，已重命名
  并同步修改 `skeleton.atlas` 中的分页文件名。
- 本资源来自同级 Unity 参考工程 `../KanadeSA`（只读工程，未做任何修改）。
- Spine 运行时版本必须与资源版本匹配：当前资源为 Spine **4.2**，对应 `rusty_spine` 0.8。
