use macroquad::prelude::*;

fn window_conf() -> Conf {
    Conf {
        window_title: "KanadeSA".to_owned(),
        window_width: 960,
        window_height: 540,
        high_dpi: true,
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    loop {
        clear_background(Color::from_rgba(24, 24, 32, 255));

        let text = "Hello, macroquad!";
        let font_size = 48.0;
        let dims = measure_text(text, None, font_size as u16, 1.0);
        draw_text(
            text,
            (screen_width() - dims.width) / 2.0,
            (screen_height() + dims.height) / 2.0,
            font_size,
            WHITE,
        );

        next_frame().await;
    }
}
