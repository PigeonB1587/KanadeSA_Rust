//! Moving diagonal stripes adapted from phira's scene/main.rs STRIPE_MATERIAL.
//! Original phira shader: GPL-3.0; see THIRD_PARTY_NOTICES.md.
use macroquad::prelude::*;

pub struct Background {
    material: Material,
}

impl Background {
    pub fn new() -> Self {
        let material = load_material(
            ShaderSource::Glsl {
                vertex: VERTEX,
                fragment: FRAGMENT,
            },
            MaterialParams {
                uniforms: vec![
                    UniformDesc::new("time", UniformType::Float1),
                    UniformDesc::new("width", UniformType::Float1),
                    UniformDesc::new("light", UniformType::Float1),
                ],
                ..Default::default()
            },
        )
        .expect("Failed to create striped background material");
        Self { material }
    }
    pub fn draw(&self) {
        self.material.set_uniform(
            "time",
            ((get_time() * 0.025) % std::f64::consts::TAU) as f32,
        );
        self.material.set_uniform("width", screen_width());
        self.material
            .set_uniform("light", super::widgets::paint::light_amount());
        gl_use_material(&self.material);
        draw_rectangle(
            0.0,
            0.0,
            screen_width(),
            screen_height(),
            super::widgets::paint::surface(Color::from_rgba(24, 24, 32, 255)),
        );
        gl_use_default_material();
    }
}

const VERTEX: &str = r#"#version 100
attribute vec3 position;
attribute vec2 texcoord;
attribute vec4 color0;
varying lowp vec4 color;
varying highp vec2 pos;
uniform mat4 Model;
uniform mat4 Projection;
void main() {
    gl_Position = Projection * Model * vec4(position, 1.0);
    color = color0 / 255.0;
    pos = position.xy;
}"#;

const FRAGMENT: &str = r#"#version 100
precision highp float;
varying lowp vec4 color;
varying highp vec2 pos;
uniform float time;
uniform float width;
uniform float light;
void main() {
    vec2 p0 = pos * (2.0 / width);
    float w = sin(0.66) * p0.y + cos(0.66) * p0.x - time;
    float p = step(mod(w, 0.02), 0.012) * mix(0.04, 0.10, light);
    vec3 stripe = mix(vec3(1.0), vec3(0.35), light);
    gl_FragColor = vec4(mix(color.rgb, stripe, p), 1.0);
}"#;
