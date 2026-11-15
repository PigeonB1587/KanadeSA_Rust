//! Gaussian rectangle shadows for elevated GUI surfaces, in logical pixels.
use macroquad::{
    miniquad::{BlendFactor, BlendState, BlendValue, Equation},
    prelude::*,
};
use std::cell::RefCell;

thread_local! {
    static MATERIAL: RefCell<Option<Option<Material>>> = const { RefCell::new(None) };
}

#[derive(Clone, Copy)]
pub enum Elevation {
    Panel,
    Popup,
}

pub fn surface(rect: Rect, color: Color, elevation: Elevation) {
    draw(rect, elevation);
    super::widgets::paint::draw_rectangle(rect.x, rect.y, rect.w, rect.h, color);
}

pub fn draw(rect: Rect, elevation: Elevation) {
    if rect.w <= 0.0 || rect.h <= 0.0 {
        return;
    }
    let light = super::widgets::paint::light_amount();
    let (sigma, drop, strength) = match elevation {
        Elevation::Panel => (4.0, 2.0, 0.30 + (0.13 - 0.30) * light),
        Elevation::Popup => (8.0, 4.0, 0.46 + (0.22 - 0.46) * light),
    };
    let offset = super::widgets::paint::offset();
    let rect = Rect::new(rect.x + offset.x, rect.y + offset.y, rect.w, rect.h);
    MATERIAL.with(|slot| {
        let mut slot = slot.borrow_mut();
        if slot.is_none() {
            *slot = Some(
                load_material(
                    ShaderSource::Glsl {
                        vertex: VERTEX,
                        fragment: FRAGMENT,
                    },
                    MaterialParams {
                        pipeline_params: PipelineParams {
                            color_blend: Some(BlendState::new(
                                Equation::Add,
                                BlendFactor::Value(BlendValue::SourceAlpha),
                                BlendFactor::OneMinusValue(BlendValue::SourceAlpha),
                            )),
                            alpha_blend: Some(BlendState::new(
                                Equation::Add,
                                BlendFactor::One,
                                BlendFactor::OneMinusValue(BlendValue::SourceAlpha),
                            )),
                            ..Default::default()
                        },
                        uniforms: vec![
                            UniformDesc::new("bounds", UniformType::Float4),
                            UniformDesc::new("blur", UniformType::Float1),
                            UniformDesc::new("drop", UniformType::Float1),
                            UniformDesc::new("ink", UniformType::Float4),
                        ],
                        ..Default::default()
                    },
                )
                .map_err(|error| tracing::warn!("Cannot load GUI shadows: {error}"))
                .ok(),
            );
        }
        let Some(material) = slot.as_ref().and_then(|value| value.as_ref()) else {
            return;
        };
        material.set_uniform("bounds", vec4(rect.x, rect.y, rect.right(), rect.bottom()));
        material.set_uniform("blur", sigma);
        material.set_uniform("drop", drop);
        material.set_uniform(
            "ink",
            vec4(
                0.018 + 0.12 * light,
                0.022 + 0.09 * light,
                0.035 + 0.06 * light,
                strength * super::widgets::paint::opacity(),
            ),
        );
        gl_use_material(material);
        let padding = sigma * 3.0 + drop;
        draw_rectangle(
            rect.x - padding,
            rect.y - padding,
            rect.w + padding * 2.0,
            rect.h + padding * 2.0,
            WHITE,
        );
        gl_use_default_material();
    });
}

const VERTEX: &str = r#"#version 100
attribute vec3 position;
uniform mat4 Model;
uniform mat4 Projection;
varying highp vec2 point;
void main() {
    point = position.xy;
    gl_Position = Projection * Model * vec4(position, 1.0);
}
"#;

const FRAGMENT: &str = r#"#version 100
precision highp float;
varying highp vec2 point;
uniform vec4 bounds;
uniform float blur;
uniform float drop;
uniform vec4 ink;
vec2 normalCdf(vec2 x) {
    vec2 t = 1.0 / (1.0 + 0.2316419 * abs(x));
    vec2 tail = 0.39894228 * exp(-0.5 * x * x) * t *
        (0.31938153 + t * (-0.35656378 + t *
        (1.78147794 + t * (-1.82125598 + t * 1.33027443))));
    return mix(tail, 1.0 - tail, step(vec2(0.0), x));
}
void main() {
    vec2 p = point - vec2(0.0, drop);
    vec2 coverage = normalCdf((p - bounds.xy) / blur) -
        normalCdf((p - bounds.zw) / blur);
    // Keep the surface itself clean, including translucent popup fills.
    float inside = step(bounds.x, point.x) * step(point.x, bounds.z) *
        step(bounds.y, point.y) * step(point.y, bounds.w);
    gl_FragColor = vec4(ink.rgb, ink.a * coverage.x * coverage.y * (1.0 - inside));
}
"#;
