# 动态饰品与催眠眼镜排查

## 来源与范围

2026-11-15，只读检查本机安装的原版游戏：

```text
/home/pigeonb1587/.steam/steam/steamapps/common/Super Animal Royale/
```

从 `Super Animal Royale_Data/resources.assets` 的 TextAsset 对象直接读取配置和骨骼，未启动游戏、注入进程或改动安装目录。Unity 参考工程 `../KanadeSA` 仅用于只读核对。

| 对象 | Path ID | 对象文件偏移（十进制） | 内容大小 |
| --- | ---: | ---: | ---: |
| `_glasses` | 10722 | 27227344 | 45450 bytes |
| `Character.skel` | 10763 | 50757184 | 11959764 bytes |

原版 `Character.skel` 与项目 `assets/spine/characters/character_01/skeleton.skel.bytes` 的 SHA-256 完全相同：

```text
f66e0475859f077326631c36f4348fb2d11afca2959e07f785f615328b53649e
```

原版 `_glasses` 的 SHA-256 为 `dd66a26b7a43ffb271599e5a741c9163a2c7dd9ca674b3e03273ed500a802eb2`。它与项目 `_glasses.json` 解析后的 160 项数据完全一致（项目 JSON 已压缩，文件字节不要求相同）。

## 直接读出的证据

催眠眼镜的 inventory ID 为 `GlassesPinwheelAnimated`，配置内容：

```json
{
  "inventoryID": "GlassesPinwheelAnimated",
  "icon": "UI/InventoryIcons/glasses_pinwheel_animated",
  "useCosmeticsAnimTrack": true,
  "spineParts": [{
    "skin": "default",
    "slot": "costumes_glasses",
    "template": "glasses_pinwheel_animated",
    "region": "extras/glasses_pinwheel_animated"
  }]
}
```

这不是需要另外拼装左右镜片的多个插槽：模板是一个加权网格，镜框顶点绑定 `glasses`（bone 221），两片镜片顶点分别绑定 `rotate_R`（222）和 `rotate_L`（223）。这些骨骼不要求额外启用 skin bones。

骨骼中已有 `misc/cosmetics` 动画，时长 2 秒，共 271 条 timeline。其中两条旋转轨道分别控制 `rotate_L`、`rotate_R`，关键帧相同：

| 时间（秒） | 相对旋转角（度） |
| ---: | ---: |
| 0 | 0 |
| 0.6666667 | −120 |
| 1.3333334 | −240 |
| 2 | −360 |

用既有 Spine runtime 对原版提取的骨骼作离线采样：

| 时间（秒） | 只播放 Idle：两镜片角度 | Idle 叠加 cosmetics：两镜片角度 |
| ---: | ---: | ---: |
| 0 | 0° | 0° |
| 0.5 | 0° | −90° |
| 1 | 0° | −180° |
| 1.5 | 0° | −270° |
| 2 | 0° | 0°（循环） |

因此，原版资产在默认播放速度下定义了每 2 秒一圈的镜片旋转；镜框保持在原骨骼上。屏幕上的旋转方向还受角色镜像和坐标系影响。

同一动画还包含帽子、项链、近战旗帜等饰品的骨骼或网格变形轨道。原版眼镜配置中标记 `useCosmeticsAnimTrack` 的还有 `GlassesSteampunkCog`。

## 缺失环节与实现

排查前 Rust 和 Unity 参考工程都读取了 `useCosmeticsAnimTrack`，但仅用于物品网格的闪光标记；现有动作预设没有播放 `misc/cosmetics`。所以贴图和关键帧都在，镜片骨骼仍停留在 setup pose。

Rust 侧增加自动装饰层：

- 根据实际穿着的服饰、当前可见武器和伞的标记启停 `misc/cosmetics`，不依赖物品名称硬编码。
- 使用独立轨道（内部索引 9），位于动作轨道之上，不占用九个可保存的动作轨道，也不改变预设的 control track。
- 装饰层默认以 1 倍速度循环，遵循角色的全局暂停。动作控制轨道的速度/循环属性仍只控制该动作轨道。
- `animation.cosmetics_time` 保存独立进度；复制、项目恢复与切换动作保留它，进入独立 Preview 时从 0 开始。
- 手动 seek 同步装饰层到所选时间；GIF 录制副本从 0 开始，按录制固定步长推进。

另一个独立问题是网格复制后失去 deform timeline 的身份关联：当前 vendored runtime 的 `spVertexAttachment_copyTo` 复制顶点与骨骼，但不会复制 `timelineAttachment`；deform timeline 又要求它与原始附件一致。换装重映射现保留模板的 `timelineAttachment`，几何和 UV 仍由各实体独立持有；播放前恢复实际挂件，避免第一帧或暂停状态下对错误附件求值。这个修复针对蛇帽、项链等网格变形，镜片旋转本身由骨骼轨道解决。

## 确认边界

这些结论来自原版配置、完整骨骼和离线采样，并非完整还原原版 C# 控制器。`GameAssembly.dll` 的 IL2CPP 导出名称与 `global-metadata.dat` 文件头经过混淆，常规入口不可直接使用；本次未解出原版调用函数、原版轨道编号或原版播放速度设置。内部轨道 9 和编辑器暂停/seek 策略是本项目的实现选择。

新增回归测试覆盖镜片旋转、暂停/seek/恢复、多实体隔离及项链 deform 关联；本轮未运行这些测试、应用构建或 GUI，也未更新此前发布的 Linux 包。已执行的离线骨骼采样只用于分析原版资产。

Spine 官方资料可作为机制参考：Applying Animations（轨道叠加）与 Skins / Inherit deform（沿用变形关键帧）。具体 SAR 动画名和关键帧以本机提取结果为依据。
