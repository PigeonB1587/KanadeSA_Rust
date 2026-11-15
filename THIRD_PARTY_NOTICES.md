# Third-party notices

`assets/audio/ui/button.ogg` is copied without modification from phira's
`assets/button.ogg`, used by its regular UI button feedback (`button_hit`).
It is distributed with phira under the project's GPL-3.0 license;
see `licenses/phira-GPL-3.0.txt`.

`assets/fonts/SourceHanSans/Regular.otf` and `Bold.otf` are Source Han Sans fonts
copied from the read-only Unity reference project's `Assets/Core/Fonts/Source Han Sans/`.
Source Han Sans is an Adobe open-source font family under the SIL Open Font License 1.1.

`assets/icons/ui/shopsparkle.png` is copied without modification from the read-only
Unity reference project's `Assets/Core/Texture2D/UGUI/ShopSparkle.png`.
It is used for the same inventory marker as Unity's `InventoryItem.SparkleImage`;
visibility follows `SkinGenerator.GetEnableSparkle` (`useCosmeticsAnimTrack`).

`src/gui/background.rs` adapts the diagonal stripe shader in
`phira/phira/src/scene/main.rs` (`STRIPE_MATERIAL` and `shader`), from
the phira project. The source project's license is GNU GPL version 3.
The adapted shader retains its stripe angle, spacing, duty cycle
and movement speed, with screen-coordinate normalization and a reduced
brightness increment (4% instead of 7%) in Dark mode. Day mode blends
toward gray stripes (10%) using the shared theme transition.
The GPL license text is included in `licenses/phira-GPL-3.0.txt`.

`src/gui/notifications.rs` adapts phira's `prpr/src/ui/billboard.rs`
(`MessageKind`, `Message`, `BillBoard`) and its message enqueue API in
`prpr/src/scene.rs`, under GNU GPL version 3. It retains the message colors,
right-side cubic slide, stacked-position smoothing, icon strip and lifetime
bar. The adaptation uses this project's logical-pixel macroquad rendering,
vector icons, bounded queues, repeated-message suppression, multiline text
and an additional fade-out, without phira's scene/UI framework dependencies.
