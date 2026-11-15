# Windows GNU 资源键修复与发布

## 路径约定

资源逻辑键使用正斜杠 `/`，与宿主系统无关；实际文件读写仍通过
`Path` / `PathBuf` 解析。不要把 `PathBuf::to_string_lossy()` 的结果用作
资源元数据索引键：Windows 会输出反斜杠，与配置中的逻辑键不匹配。

`src/assets.rs::logical_key()` 只规范逻辑键的分隔符，不改写用户项目、
输出文件或其他磁盘路径。`src/booth.rs::sprite_metadata()` 负责统一加载
元数据和缓存相邻图片，供阴影、水印、背景、场景图层和 GEM 共用；
`resource_path()` 同时供背景与场景缩略图使用。

已检查其他字符串缓存：图标缓存使用配置 ID，项目缩略图缓存使用项目 ID，
Spine 使用 atlas 相邻磁盘文件，没有相同的原生路径转资源键问题。

## 回归验证

```sh
cargo fmt --all -- --check
cargo test --locked
cargo build --release --locked --target x86_64-pc-windows-gnu
```

回归测试显式构造 Windows 反斜杠和混合分隔符，不依赖宿主的
`Path::join()`。测试调用与生产加载一致的元数据缓存函数，覆盖阴影、
水印、全部 100 个 GEM、15 个背景及其缩略图和图层，检查规范键、
缓存复用、图片尺寸及缺失元数据报错。

## 发布检查

- 从 Cargo.toml 读取真实版本，在发布目录创建独立的
  `KanadeSA_Rust-v<版本>-windows-x86_64-gnu` 文件夹；冲突时使用明确后缀。
- 仅复制本次构建的 EXE、完整 assets、许可证、来源及说明；不复制用户数据。
- FFmpeg 使用重新核验 SHA-256 的 Windows x64 LGPL 静态包，保留许可证
  及镜像来源。启动 BAT 使用 `cd /d "%~dp0"` 和局部 `PATH`。
- 检查 EXE 导入 DLL、全部资源哈希、sprites.json / atlas 引用、ZIP CRC
  与顶层目录，并比较包内 EXE 与构建产物的 SHA-256。
- 没有 Windows 或 Wine 时，明确记录没有做 Windows 实际运行验证；
  GNU 交叉编译通过不代表运行验证通过。
