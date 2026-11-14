use rusty_spine::{Atlas, Physics, Skeleton, SkeletonJson};
use std::sync::Arc;

#[test]
fn no_scale_bones_ignore_parent_scale_and_use_animated_inherit_mode() {
    let atlas = Arc::new(Atlas::new(b"", "").unwrap());
    // The parent is reflected, rotated, and scaled differently on each axis.
    let json = br#"{"skeleton":{"spine":"4.2.43"},"bones":[
        {"name":"root","rotation":27,"scaleX":-0.25,"scaleY":0.6},
        {"name":"head","parent":"root","rotation":35,"scaleX":1.3,"scaleY":0.8}
    ]}"#;
    let data = Arc::new(SkeletonJson::new(atlas).read_skeleton_data(json).unwrap());
    let mut skeleton = Skeleton::new(data);
    for (mode, reflected) in [(3, true), (4, false)] {
        // Change the current inherit mode while setup data stays Normal, like an inherit timeline.
        unsafe {
            (*skeleton.find_bone_mut("head").unwrap().c_ptr()).inherit = mode;
        }
        skeleton.update_world_transform(Physics::Pose);
        let head = skeleton.find_bone("head").unwrap();
        assert!((head.world_scale_x() - 1.3).abs() < 0.0001);
        assert!((head.world_scale_y() - 0.8).abs() < 0.0001);
        let determinant = head.a() * head.d() - head.b() * head.c();
        assert_eq!(determinant < 0.0, reflected);
    }
}
