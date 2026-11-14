# KanadeSA_Rust

使用 macroquad 渲染 Spine 角色的 Rust 照相亭，支持多角色独立换装、装备/宠物、分轨动作、多场景、透明拍照和保存恢复。

角色资源来自同目录下的 Unity 参考工程 `../KanadeSA`（只读），默认展示参数与参考工程保持一致：

- Spine 运行时：[`rusty_spine`](https://crates.io/crates/rusty_spine) 0.8（由官方 Spine C Runtime 转译，支持 **Spine 4.2**；当前资源为 `4.2.43`）
- 默认角色/皮肤：`Fox/-Fox`（Unity 预制体 `Character Local` 中的 `initialSkinName`）
- 默认动画：`idle/idle`，九种多轨动作预制内嵌于 `src/model.rs`，运行时不读取 `_animation.json`
- 共享 AnimationStateData 的默认混合时间为 `0.2s`；逐轨重配、initial→loop 和交替射击使用零混合，确保暂停、seek 与保存恢复保持确定性
- 照相亭使用固定 16:9 取景，对应 Unity 相机 `orthographic size = 5`、骨骼导入缩放 `0.01`、`basePlayerScale = 0.9`，角色默认位置为 `(0, -3)`；Preview 沿用先前的角色高度比例（窗口高度的 42%）和底部留白（20%），仅在附件、宠物或 GEM 超出预览区时缩小，不放大填满区域

## 运行

```bash
source ~/.cargo/env   # 如果还没把 cargo 加入 PATH
cargo run
```

首次运行需要从 crates.io 下载并编译依赖。`Cargo.toml` 中为调试构建的依赖开启了优化（`opt-level = 2`，`rusty_spine` 为 3）：否则解析 12 MB 骨骼文件、解码 4096×4096 PNG 会非常慢。

启动后进入照相亭；`Preview` 打开所选角色的上层编辑窗口，底部 `Back to booth` 或 `Esc` 返回。关闭系统窗口退出应用。
每次打开 Preview 都从 Idle 开始，不继承照相亭的动作、暂停状态或轨道进度；角色及宠物的镜像、位置、缩放、旋转、瞄准角度和颜色使用独立默认值。Preview 内主动切换动作或试用装备不影响照相亭动作和场景属性。
Preview 完整叠在照相亭 GUI 上方，底层只绘制、不响应输入；底板与面板采用照相亭同款深灰蓝色，全屏底板使用 30% 透明度（70% 不透明度），服装列表与信息卡保留 98% 不透明度以保证文字可读性。

## 照相亭

- 顶栏 `+ Role` / `- Role` 新增或删除角色。新增角色复制当前外观并分配新 ID；每个实体拥有独立的骨骼控制器、装备、枪械皮肤、等级、动作与场景属性。底部 `<` / `>` 或画布点击可切换角色。
- 直接拖动角色调整地面位置和深度；拖动宠物调整其位置。较近的角色、宠物与场景物体按各自深度遮挡，影子独立位于底层。武器/表情按钮位于舞台外工具行；Save / Load / Photo 在顶部靠右，与顶栏按钮同款，小窗口在顶部第二行靠右排列。
- `Transform` 用左右位置、前后远近和离地高度表示角色位置（场景单位；远近数值越大越远，离地高度 0 表示落地），并调整大小、旋转、TARGET 相对角度、影子透明度、角色/宠物独立镜像、宠物参考系、颜色与等级。内部仍保存原始世界坐标；调整远近保持离地高度，调整离地高度不改变深度。
- 角色使用场景世界坐标。宠物可使用角色参考系（相对角色容器的位移，乘角色所处深度的透视系数）或世界参考系（绝对坐标）；切换参考系时保留当前世界位置，宠物大小按自己的世界 Y 计算。
- `Animation` 仅提供 Unity 开发者配置中的九种固定预设，不开放任意轨道编辑、骨骼片段选择或控制轨道切换。支持暂停/播放及进度、速度和循环控制，始终作用于预设固定的 controlTrack（Emote 为 1，其余为 0，均为零基索引）。预设及 controlTrack 内嵌在 Rust 代码中，不运行时读取动画 JSON。预设轨道随实际装备解析；换装保留不受影响轨道的进度。
- 画布左上角的武器/表情图标打开实际选择器。武器按该角色在 Preview 配好的近战外观和各枪皮肤显示，白色图标保留原 alpha 轮廓；选择后实际持有，非持械预制会回到 Idle。表情可以直接使用 Preview 配置项或从原始列表选择，自动进入 Emote 预制。列表不合成额外的 None/空手选项。
- `Scene` 切换 15 个背景、左/右/隐藏水印与绿布透明输出，也可修改保存目录。场景按配置层次和图片导入元数据还原；不使用 Unity GUI。
- 普通取景缓冲为 1280×720；`Photo` 或照相亭中的 `F12` 保存 1920×1080 PNG 到 `photos/`，排除所有 GUI。绿布开启透明后导出透明背景和半透明边缘；非透明照片保留不透明 alpha。

`Save` / `Load` 默认读写 `saves/default/`，存在当前场景时启动自动恢复：

- `characters.json`：稳定角色 ID、动物/服饰 inventory ID、装备、每种枪的皮肤名、`equipment.loadout` 搭配配置（近战和表情）、实际 `weapon` / `emote`、等级、各轨道当前片段/时间/速度/循环、暂停状态和宠物动画时间。旧存档缺少 loadout 时从原选择补全，保留实际武器、表情和动画状态。
- `scene.json`：背景、水印、透明开关、选中角色及通过 `character_id` 引用的场景变换，不嵌入完整角色数据。

两份 JSON 具有相同 revision；保存先写入并同步 `.pending`，再替换当前文件，保留完整 `.previous` 配对供中断后恢复。加载先校验并完整构建新实体，再替换现场；无效资源或动作不会清空当前场景。空场景也可以保存恢复。不要将不同次保存的两份 JSON 混用。

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
应用内弹窗共用 `src/gui/widgets.rs`：0.28 秒三次缓出、60 设计单位纵向滑入/退出，面板、文字、图标与选中底色同步淡入淡出；中途反向保持位置和透明度连续。关闭帧保留完整内容，退场结束前持续拦截底层输入。Preview 本身也使用相同过渡，角色及宠物只在渲染时淡出，不改写持久颜色。
场景、动画预设、武器/表情、设置、紧凑 Info、分类共用标题栏与深色矩形底、粗体文字的 `Close` 按钮，保留 Esc 和点击背景关闭；所有关闭途径只播放一次 click 音效，面板内无操作的空白区域不播放。语言下拉列表使用独立的无标题栏列表，不显示 Language/Close，优先响应 Esc，不连带关闭设置。所有列表统一使用滚轮/触摸拖动识别和 framebuffer 遮罩裁剪，允许边缘显示部分行，点击限制在可见区域。系统原生输入框仍由操作系统管理。
应用版本、数字版本 ID、Spine 素材来源游戏版本集中定义在 `src/version.rs`，仅在设置面板显示。
来源游戏版本尚未从素材中确认，当前为 `Unknown`；它与 Spine 编辑器版本不同。

右侧为鼠标选取面板：点击分类后直接点击方形图标应用角色或装扮。
`Neck` 分类对应 `_neck.json`。分类按钮展开平级分类列表，搜索框按当前语言名称或物品 ID 筛选，
忽略大小写，支持空格分隔多个关键词；点击 `x` 清空。系统输入框或分类菜单打开时暂停底层快捷键，
确认/取消由系统输入框处理，点击列表即可直接选择搜索结果。
鼠标悬停显示物品名称，橙色边框表示当前选择；在网格上滚动滚轮或拖动右侧滚动条浏览。
`Clear all` 卸下五类服饰，顶部 `<` / `>` 可用鼠标切换动作。原有键盘操作在 Preview 中继续可用。

装备采用平级分类：`Weapon-Melee`、`Weapon-AK47`、`Weapon-UZI` 等，不提供投掷物分类入口；
每种枪单独分类，分类内选择皮肤，不再合成枪械 None。`Umbrella`、`Pet` 和 `Emote` 独立选择。
所有配置列表保留原始顺序和原有空项，不人为插入 None；默认选择第一项（伞为红伞，宠物第一项为配置原有的空宠物）。新角色的实际近战和表情也默认第一项，旧存档保留已保存的实际选择。
Preview 使用共享资源但独立控制器的试穿实体；武器/表情选择只编辑搭配并在 Preview 试用，不改变照相亭当前武器、表情或动作进度。所有枪皮分别保留选择，切换到另一把枪不会丢失配置；Emotes 严格保留原 JSON 的 74 项顺序，第一项为默认舞蹈，不插入 None。返回后通过照相亭图标选择实际使用内容。
图标路径为 `icons/{配置 icon 转小写}.png`；枪械按 `weapon.icon + skinNames[index]` 拼接。缺省、假路径或加载失败统一采用 Unity 的 `UI/InventoryIcons/item_rejection_letter`，不会以 None 圆叉代替无图标物品。
特殊伞通过独立 atlas 重映射模板，GEM 随等级变化；宠物继承角色颜色，不提供点击触摸动作彩蛋。

图标位于 `icons/ui/inventoryicons/`，仅包含从 Unity Resources 按配置路径复制的 PNG，
文件路径统一小写。列表仅绘制可见单元格，每帧最多加载两张图标，
使用最多 128 张纹理的最近使用缓存；移出缓存的纹理由 macroquad 释放。

GUI 使用 `1280×720` 设计单位和阶梯缩放：`1 / 1.25 / 1.5 / 1.75 / 2 / 2.5 / 3`，
按窗口宽高能容纳的最大档位选择，不连续缩放，也不把小窗口控件缩到 1 倍以下。
字体、间距、按钮与鼠标命中测试共用这一坐标系，选取网格仍按可用空间调整行列。
逻辑宽度小于 1000 或高度小于 560 时使用紧凑布局：隐藏皮肤路径、装备摘要和冗长提示，
选取列表始终展开：一般窗口左右分区，宽度不足 620 逻辑单位时通常改为上方预览、下方列表；短宽窗口仍保留左右分区并压缩列表顶部。
角色、附件与宠物按独立预览区域适配，保留镜像与旋转。选中物品不会关闭列表。
照相亭在短宽窗口使用左右分区，窄竖窗口使用上下分区；属性内容可滚动，按钮轻触松开确认。

紧凑布局顶部 `info` 文字按钮可展开/收起 Skin、动画序号和装备摘要；高度不足时可在信息面板内滚动。
背景使用移植自 phira 的缓慢斜向扫描条纹着色器，仅影响背景，不叠加到角色或界面上。
来源与许可证见 `THIRD_PARTY_NOTICES.md`。

从项目根目录的 `configs/` 读取 JSON 配置。角色列表以 `_characters.json` 为准，
保留配置顺序，并使用 `spineSkin`、`spineBaseSkin`、`needsCutHats` 和
`customShiaHeadband` 规则。界面名称读取 `_languages.json` 中的当前语言，缺失显示名可回退英文，搜索不回退。

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
装备另从 `_weapons.json`、`_umbrellas.json`、`_pets.json`、`_emotes.json` 读取。

## 目录结构

```
.
├── Cargo.toml
├── src
│   ├── main.rs            # 照相亭与上层 Preview 主循环
│   ├── character.rs       # 共享资源、独立角色/宠物控制器与 Preview 构图
│   ├── animation.rs       # 多轨动作解析、initial/alternate、seek 与恢复
│   ├── equipment.rs       # 平级装备类别、稳定皮肤名与附件配置
│   ├── booth.rs           # 场景层、坐标/透视、排序、影子/水印与拍照
│   ├── model.rs           # 角色数据、场景引用与内嵌动作预制
│   ├── persistence.rs     # 双 JSON 配对、暂存与恢复
│   ├── verification.rs    # 可选真实 GPU 验证入口
│   ├── sprite_material.rs # 离屏 RGBA sprite 混合
│   ├── wardrobe.rs        # 配置目录、服装模板重映射、自定义皮肤与可见性
│   ├── gui/               # GUI 展示层
│   │   ├── mod.rs         # GUI 模块入口
│   │   ├── layout.rs      # 阶梯缩放与共享区域布局
│   │   ├── preview.rs     # 顶栏、装备摘要、Info 与底栏
│   │   ├── booth.rs       # 照相亭画布、属性、场景/动作选择
│   │   ├── equipment.rs   # 照相亭实际武器/表情选择器
│   │   ├── icons.rs       # 图标缓存、Unity 缺图回退与白化轮廓
│   │   ├── selection.rs   # 虚拟化图标网格与搜索
│   │   ├── settings.rs    # 设置模态层与语言选择
│   │   ├── drag_scroll.rs # 鼠标/触摸拖动手势
│   │   └── background.rs  # 扫描线背景
│   ├── version.rs         # 设置面板专用版本信息
│   ├── logging.rs         # 控制台日志和按天轮转的日志文件
│   └── spine_renderer.rs  # rusty_spine -> miniquad 渲染器（着色器/混合模式/纹理）
├── configs                # 角色、服饰、装备、宠物、场景与多语言配置
├── booth                  # 场景/GEM/影子/水印及各目录 sprites.json
├── tools/import_unity_booth.py # 只读参考工程的资源导入脚本
└── spineAssets
    ├── README.md
    ├── pets               # 宠物骨骼和 atlas
    ├── umbrellas          # 独立伞 PNG 和生成的 atlas
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
   - `backface_culling`：是否启用背面剔除；现有资源默认关闭
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
`vendor/rusty_spine/PATCHES.md`。本地版同时修复 `Color::clamp()` 对绿、蓝、alpha 的错误赋值。
请保留 vendor 目录，`cargo test` 包含继承与颜色回归测试。

macroquad 自带的 `Material`/`draw_mesh` 只支持 `position + uv + color` 顶点和固定混合模式，而 Spine 需要额外的 `dark_color` 顶点属性以及 Normal/Additive/Multiply/Screen 四种混合模式。因此 `spine_renderer.rs` 通过 `macroquad::window::get_internal_gl()` 直接提交 miniquad 绘制调用，同时继续用 macroquad 的 `Texture2D` 管理纹理生命周期。

贴图采用**首次使用时才加载**的策略：当前角色的 atlas 包含 10 张 4096×4096 贴图，按需加载可避免一次性占用数百 MB 显存。

不可变 skeleton/atlas、配置及语言目录通过 `Arc` 共享。原始角色 mesh 保留共享身份以维持 deform；
region 模板单独复制，外部伞 mesh 使用 linked clone。实体 controller 必须先于其 custom skin 释放。

## 资源导入与验证

已有资源可直接运行，无需 Unity。需要重新同步只读参考资源时，在本工程运行：

```bash
python3 tools/import_unity_booth.py ../KanadeSA
cargo fmt --all -- --check
cargo check --quiet
cargo test --quiet
cargo run --quiet -- --verify-booth /tmp/opencode/booth-verify
```

导入脚本仅向本工程复制图片/配置并生成元数据，不写入参考工程。`booth/xxx/sprites.json` 保存原图尺寸、PPU、
pivot、过滤、mipmap、色彩空间及其他导入设置；渲染按 PPU/pivot 转换坐标。
Unity 世界 Y 向上、相机半高 5；Booth 配置单位换算为 `5 / 67.5`，物体深度为 `(offsetY + zOffset) × 5 / 67.5`。

GPU 验证需图形会话，使用实际渲染和保存路径，输出各场景照片、透明 PNG、保存恢复前后图像、
多种窗口尺寸下的照相亭与伞/GEM Preview。无图形环境可运行 `cargo test`，包括真实骨骼动作、
全角色/服饰、武器皮肤和伞遍历，以及独立实体、JSON 引用与恢复校验；完整遍历可能需要数分钟。
