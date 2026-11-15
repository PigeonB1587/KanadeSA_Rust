# 应用资源

随应用分发的只读资源统一放在这里；从项目根目录运行应用，分发时保持
`assets/` 与可执行文件的运行目录对应。加载入口和公共路径位于 `src/assets.rs`。

```text
assets/
├── audio/       # UI 音效
├── booth/       # 场景、缩略图、GEM、影子、水印及 sprites.json
├── configs/     # 角色、服饰、装备、语言和场景 JSON
├── fonts/       # Rubik、Source Han Sans 及字体许可证
├── icons/       # 配置指定的 UI/Inventory 图标，路径统一小写
└── spine/       # 角色、宠物、spring 平台、伞 atlas 和分页图片
```

- 配置和工程中的 `Booth/...`、`UI/...`、inventory ID 是逻辑资源标识，
  不包含 `assets/` 前缀；在加载文件时解析，不随目录整理改写。
- Spine `.atlas` 与分页贴图一起移动，分页名相对 atlas 所在目录解析，
  保留原大小写、空格和相对路径。骨骼二进制及贴图内容不变。
- `sprites.json` 留在对应图片目录，键名仍是图片文件名。
- JSON 使用紧凑格式。运行 `python3 tools/compact_json.py` 可压缩此目录下
  的 JSON；它只移除字符串之外的空白，保留原始字符串、数值文本和字段顺序。
  资源导入脚本同样生成紧凑 JSON。
- 用户的 `projects/`、`exports/`、`editor_settings.json` 和 `logs/`
  保留在应用根目录，不能混入此目录；已有用户数据不会被资源整理脚本改写。

资源来源和许可证见根目录 `THIRD_PARTY_NOTICES.md`。
