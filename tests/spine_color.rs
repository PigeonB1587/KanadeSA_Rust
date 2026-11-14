#[test]
fn spine_tint_preserves_independent_rgb_and_alpha_channels() {
    let mut color = rusty_spine::Color::default();
    color.set_from_floats(0.8, 1.0, 0.7, 1.0);
    assert_eq!([color.r, color.g, color.b, color.a], [0.8, 1.0, 0.7, 1.0]);
    color.set_from_floats(-0.1, 0.3, 1.5, 0.6);
    assert_eq!([color.r, color.g, color.b, color.a], [0.0, 0.3, 1.0, 0.6]);
}
