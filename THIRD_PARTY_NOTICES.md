# Third-party notices

`audio/ui/button.ogg` is copied without modification from phira's
`assets/button.ogg`, used by its regular UI button feedback (`button_hit`).
It is distributed with phira under the project's GPL-3.0 license;
see `licenses/phira-GPL-3.0.txt`.

`fonts/SourceHanSans/Regular.otf` and `Bold.otf` are Source Han Sans fonts
copied from the read-only Unity reference project's `Assets/Core/Fonts/Source Han Sans/`.
Source Han Sans is an Adobe open-source font family under the SIL Open Font License 1.1.

`src/gui/background.rs` adapts the diagonal stripe shader in
`phira/phira/src/scene/main.rs` (`STRIPE_MATERIAL` and `shader`), from
the phira project. The source project's license is GNU GPL version 3.
The adapted shader retains its stripe angle, spacing, duty cycle
and movement speed, with screen-coordinate normalization and a reduced
brightness increment (4% instead of 7%).
The GPL license text is included in `licenses/phira-GPL-3.0.txt`.
