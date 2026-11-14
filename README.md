# KanadeSA_Rust

使用 [macroquad](https://github.com/not-fl3/macroquad) 渲染 Spine 角色的最小 Rust 工程。

角色资源来自同目录下的 Unity 参考工程 `../KanadeSA`（只读），默认展示参数与参考工程保持一致：

- Spine 运行时：[`rusty_spine`](https://crates.io/crates/rusty_spine) 0.8（由官方 Spine C Runtime 转译，支持 **Spine 4.2**；当前资源为 `4.2.43`）
- 默认角色/皮肤：`Fox/-Fox`（Unity 预制体 `Character Local` 中的 `initialSkinName`）
- 默认动画：`idle/idle`（Unity `_animation.json` 中的 Idle 动作）
- 默认混合时间：`0.2s`（Unity `Character_SkeletonData.defaultMix`）
- 构图：对应 Unity 相机 `orthographic size = 5`、骨骼导入缩放 `0.01`、`basePlayerScale = 0.9`，角色约占窗口高度的 42%，脚底位于画面下方 20% 处（对应 Unity 默认角色位置 `(0, -3)`）

## 运行

```bash
source ~/.cargo/env   # 如果还没把 cargo 加入 PATH
cargo run
```

首次运行需要从 crates.io 下载并编译依赖。`Cargo.toml` 中为调试构建的依赖开启了优化（`opt-level = 2`，`rusty_spine` 为 3）：否则解析 12 MB 骨骼文件、解码 4096×4096 PNG 会非常慢。

操作：`←` / `→` 切换动画，通过 GUI 选择角色皮肤，`Esc` 退出。

## 换装

搜索框点击后通过与 phira 相同的 `inputbox` 异步接口打开系统输入对话框，
由系统处理中文输入法、组合输入和粘贴；确认后更新搜索，取消保留原内容。
Linux 需要 `zenity` 或 `yad`（Fedora：`sudo dnf install zenity`）。
搜索名称只匹配当前语言翻译，不使用英文回退；inventory ID 仍可跨语言查找。
移动端后端尚未实机验证：Android 打包还需接入 `inputbox-android` AAR 并初始化 JNI 后端，
不能仅启用 Rust 依赖就视为完成 Android 输入支持。

UI 点击音效使用 phira 的 `button.ogg`，位于 `audio/ui/button.ogg`，启动时加载一次。
按钮、分类、搜索框、物品选择和语言选择播放点击反馈；悬停、滚轮及拖动滚动不播放。
Linux 构建需安装 ALSA 开发库（Fedora：`sudo dnf install alsa-lib-devel`；
Debian/Ubuntu：`sudo apt install libasound2-dev`）。缺失时链接器会报告找不到 `-lasound`。

顶部 `settings` 文字按钮打开设置面板，使用约 0.28 秒的三次缓出滑入/滑出过渡。
面板背景、文字和下拉列表同步淡入淡出。语言选项从 `_languages.json` 的实际字段生成，
设置在原有界面（包括已展开的 Info）上覆盖全屏黑色底罩，不改变 Info 的开关状态。
语言下拉列表使用适配 DPI 的裁剪区域，滚动到边缘时只裁去超出部分，不整行隐藏。
已知字段显示语言名称，新增未知字段以其键名显示，不引入配置外的语言。
服饰/角色网格和语言列表均可直接按住内容上下拖动；超过 7 个逻辑单位后视为滚动，
不会触发选择。点击/轻触在松开时确认。保留滚轮与原有网格滚动条，
鼠标和触摸共用手势逻辑，为后续移动端保持可操作性（尚未进行移动端实机验证）。
目前提供语言下拉列表（配置中的 12 种语言）；切换后角色和物品名称、None 以及
当前搜索结果同步更新。日语搜索参照 Unity：NFKC 全半角标准化，并统一平假名/片假名。
其他界面标签保留现有英文；中日韩名称使用随附的 Source Han Sans 字体显示。
设置面板打开或退出动画期间拦截底层输入，`Esc` 或点击外部/close 关闭设置。
应用版本、数字版本 ID、Spine 素材来源游戏版本集中定义在 `src/version.rs`，仅在设置面板显示。
来源游戏版本尚未从素材中确认，当前为 `Unknown`；它与 Spine 编辑器版本不同。

右侧为鼠标选取面板：点击分类后直接点击方形图标应用角色或装扮。
`Neck` 分类对应 `_neck.json`。分类下方的搜索框按当前分类的英文名称或物品 ID 筛选，
忽略大小写，支持空格分隔多个关键词；点击 `x` 清空。输入时暂停全局快捷键，
`Enter` / `Esc` 结束输入，点击列表即可直接选择搜索结果。
鼠标悬停显示物品名称，橙色边框表示当前选择；在网格上滚动滚轮或拖动右侧滚动条浏览。
`Clear all` 卸下所有装扮，顶部 `<` / `>` 可用鼠标切换动画。原有键盘操作继续可用。

图标位于 `icons/ui/inventoryicons/`，仅包含从 Unity Resources 按配置路径复制的 PNG，
文件路径统一小写。列表仅绘制可见单元格，每帧最多加载两张图标，
使用最多 128 张纹理的最近使用缓存；移出缓存的纹理由 macroquad 释放。

GUI 使用 `1280×720` 设计单位和阶梯缩放：`1 / 1.25 / 1.5 / 1.75 / 2 / 2.5 / 3`，
按窗口宽高能容纳的最大档位选择，不连续缩放，也不把小窗口控件缩到 1 倍以下。
字体、间距、按钮与鼠标命中测试共用这一坐标系，选取网格仍按可用空间调整行列。
逻辑宽度小于 1000 或高度小于 560 时使用紧凑布局：隐藏皮肤路径、装备摘要和冗长提示，
选取列表始终展开：一般窗口左右分区，宽度不足 620 逻辑单位时改为上方预览、下方列表。
角色按独立预览区域居中并限制大小，选中物品不会关闭列表，也不会覆盖角色。

紧凑布局顶部 `info` 文字按钮可展开/收起 Skin、动画序号和装备摘要；高度不足时可在信息面板内滚动。
背景使用移植自 phira 的缓慢斜向扫描条纹着色器，仅影响背景，不叠加到角色或界面上。
来源与许可证见 `THIRD_PARTY_NOTICES.md`。

从项目根目录的 `configs/` 读取 JSON 配置。角色列表以 `_characters.json` 为准，
保留配置顺序，并使用 `spineSkin`、`spineBaseSkin`、`needsCutHats` 和
`customShiaHeadband` 规则。界面名称读取 `_languages.json` 中的英文翻译。

- `Tab`：循环选择面饰/颈饰、衣服、眼镜、帽子、胡子五个类别。
- `Q` / `E`：切换所选类别的上一件/下一件物品；`None` 表示卸下该类别。
- `Backspace`：卸下所有装扮。
- GUI 角色列表：更换角色，保留当前装备并按新角色的基础皮肤重建。

五个类别分别对应 `_neck.json`、`_clothes.json`、`_glasses.json`、
`_hats.json`、`_beards.json`。换装遵循 Unity `SARCharacterController`：
共享原始角色附件，自定义皮肤中仅复制服装模板并替换 atlas region，
支持 `#BASE_ANIMAL`、`includeFront`、`regionCut`、特殊头巾和头发隐藏。
换装不重启当前动画；卸下装备会重建皮肤并清理旧插槽附件。
缺失模板、插槽或贴图区域时保留原装扮并记录英文警告。
`_weapons.json` 本轮不参与五类装扮逻辑。

## 目录结构

```
.
├── Cargo.toml
├── src
│   ├── main.rs            # 窗口、主循环、HUD
│   ├── character.rs       # 角色注册表、加载、动画、构图
│   ├── wardrobe.rs        # 配置目录、服装模板重映射、自定义皮肤与可见性
│   ├── gui/               # GUI 展示层
│   │   ├── mod.rs         # GUI 模块入口
│   │   ├── layout.rs      # 阶梯缩放与共享区域布局
│   │   ├── preview.rs     # 顶栏、装备摘要、Info 与底栏
│   │   ├── selection.rs   # 虚拟化图标网格与搜索
│   │   ├── settings.rs    # 设置模态层与语言选择
│   │   ├── drag_scroll.rs # 鼠标/触摸拖动手势
│   │   └── background.rs  # 扫描线背景
│   ├── version.rs         # 设置面板专用版本信息
│   ├── logging.rs         # 控制台日志和按天轮转的日志文件
│   └── spine_renderer.rs  # rusty_spine -> miniquad 渲染器（着色器/混合模式/纹理）
├── configs                # 角色、五类装扮、英文物品名称配置
└── spineAssets
    ├── README.md
    └── characters
        └── character_01
            ├── skeleton.atlas
            ├── skeleton.skel.bytes
            └── skeleton*.png
```

## 资源整理约定

`spineAssets/characters/<角色目录>/` 下每个角色保持一套统一命名：

- `skeleton.atlas`：Spine 导出的 atlas 文件
- `skeleton.skel.bytes`：Spine 二进制骨骼
- `skeleton.png`、`skeleton_2.png`、...：atlas 中引用的分页贴图

atlas 文件中的贴图名必须与实际文件名一致（本项目的 `skeleton.atlas` 已从原始 `Character_full*.png` 重命名并同步更新）。

## 添加新角色

1. 在 `spineAssets/characters/` 下新建目录，例如 `character_02`，放入整理好的 `skeleton.atlas`、`skeleton.skel.bytes` 和贴图。
2. 在 `src/character.rs` 的 `CHARACTERS` 数组中追加一项 `CharacterConfig`：
   - `atlas_path` / `skeleton_path`：资源路径
   - `skin`：要使用的 Skin。注意本资源的 `default` Skin 只有一个占位附件，必须指定完整 Skin（如 `Fox/-Fox`）
   - `animation`：起始动画，不存在时会自动回退到 `idle/idle` 或第一个可用动画
   - `scale`：`None` 时按 `height_ratio` 自动适配高度；需要精确还原 Unity 构图时可填固定缩放
3. `cargo run` 即可。

若新骨骼使用不同的角色/服饰目录，需要同步提供 `configs/` 中相应配置。
运行时只展示 `spineSkin` 和 `spineBaseSkin` 都存在于该骨骼中的角色。

## 实现说明

### 日志

运行时使用 `tracing`，控制台输出本地时间戳、级别、模块和消息，交互式终端启用颜色。
平均 FPS 和平均帧耗时每秒记录一次，所有运行时日志消息均使用英文。
日志同时由后台线程写入 `logs/kanade_sa.YYYY-MM-DD.log`（按 UTC 日期轮转），
保留最近 7 份；`logs/` 已加入 Git 忽略。正常退出会刷新文件队列，panic 会记录到日志。

默认日志级别为 `info`，可通过环境变量调整：

```bash
RUST_LOG=debug cargo run
RUST_LOG=info,kanade_sa::performance=warn cargo run # 隐藏每秒性能日志
```

通过 `[patch.crates-io]` 使用 `vendor/rusty_spine` 中的 0.8.0 本地修正版。
原版 `NoScale / NoScaleOrReflection` 骨骼变换缺少父级方向向量归一化，
会使不继承缩放的骨骼仍随父级缩小（如 `menu/lab_embryo` 的头部）。
修复与 Unity 参考 Spine 插件 `Bone.UpdateWorldTransform` 对齐；详见
`vendor/rusty_spine/PATCHES.md`。请保留 vendor 目录，`cargo test` 包含缩放继承回归测试。

macroquad 自带的 `Material`/`draw_mesh` 只支持 `position + uv + color` 顶点和固定混合模式，而 Spine 需要额外的 `dark_color` 顶点属性以及 Normal/Additive/Multiply/Screen 四种混合模式。因此 `spine_renderer.rs` 通过 `macroquad::window::get_internal_gl()` 直接提交 miniquad 绘制调用，同时继续用 macroquad 的 `Texture2D` 管理纹理生命周期。

贴图采用**首次使用时才加载**的策略：当前角色的 atlas 包含 10 张 4096×4096 贴图，按需加载可避免一次性占用数百 MB 显存。
