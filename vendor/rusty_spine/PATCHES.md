# Local fixes to rusty_spine 0.8.0

Compiler compatibility: add explicit `'_` to borrowed return-type lifetimes,
use raw addresses for C static arrays, copy the optional debug allocator before
matching it, and make the read-only character lookup table immutable. Singleton
slots are accessed through explicit raw addresses after `Once` initialization;
their initialization and callback behavior are unchanged. These edits address
the `mismatched_lifetime_syntaxes` and `static_mut_refs` warnings without
disabling compiler lints.

Source: crates.io rusty_spine 0.8.0. Original license is retained in LICENSE.

In src/c/spine_c.rs, spBone_updateWorldTransformWith now matches the
NoScale / NoScaleOrReflection branch in the read-only Unity reference's
Assets/Spine/Runtime/spine-csharp/Bone.cs, lines 265–290:

- Normalize the parent-transformed direction before constructing the basis.
  The original implementation leaked parent scale into NoScale bones.
- Test the current bone's animated inherit mode, rather than its setup-data
  mode, when deciding whether to retain a reflection.

This fixes bone transforms before constraints and mesh generation; it does
not apply character-specific scale compensation in the renderer.

In src/color.rs, `Color::clamp()` now clamps each channel from its own value.
The upstream implementation accidentally assigned clamped red to green, blue
and alpha, turning independent tints into grayscale and making a red tint below
one unexpectedly translucent. `tests/spine_color.rs` in the application checks
independent RGBA values and out-of-range clamping.
