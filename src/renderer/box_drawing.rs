pub fn fill_box_drawing(c: char, buf: &mut [u8], cw: u32, ch: u32) {
    let mid_x = cw / 2;
    let mid_y = ch / 2;
    let thick = (ch / 14).max(1); // Grosor proporcional

    let draw_horiz = |buf: &mut [u8], x0: u32, x1: u32, y: u32| {
        for x in x0..x1.min(cw) {
            for t in 0..thick {
                let py = (y + t).min(ch - 1);
                buf[(py * cw + x) as usize] = 255;
            }
        }
    };

    let draw_vert = |buf: &mut [u8], x: u32, y0: u32, y1: u32| {
        for y in y0..y1.min(ch) {
            for t in 0..thick {
                let px = (x + t).min(cw - 1);
                buf[(y * cw + px) as usize] = 255;
            }
        }
    };

    match c {
        // Líneas simples
        '─' => draw_horiz(buf, 0, cw, mid_y),
        '│' => draw_vert(buf, mid_x, 0, ch),
        // Esquinas (ajustadas para unión perfecta)
        '╭' => {
            draw_horiz(buf, mid_x, cw, mid_y);
            draw_vert(buf, mid_x, mid_y, ch);
        }
        '╮' => {
            draw_horiz(buf, 0, mid_x + thick, mid_y);
            draw_vert(buf, mid_x, mid_y, ch);
        }
        '╰' => {
            draw_horiz(buf, mid_x, cw, mid_y);
            draw_vert(buf, mid_x, 0, mid_y + thick);
        }
        '╯' => {
            draw_horiz(buf, 0, mid_x + thick, mid_y);
            draw_vert(buf, mid_x, 0, mid_y + thick);
        }
        // Cruces y Tes
        '├' => {
            draw_vert(buf, mid_x, 0, ch);
            draw_horiz(buf, mid_x, cw, mid_y);
        }
        '┤' => {
            draw_vert(buf, mid_x, 0, ch);
            draw_horiz(buf, 0, mid_x, mid_y);
        }
        '┬' => {
            draw_horiz(buf, 0, cw, mid_y);
            draw_vert(buf, mid_x, mid_y, ch);
        }
        '┴' => {
            draw_horiz(buf, 0, cw, mid_y);
            draw_vert(buf, mid_x, 0, mid_y);
        }
        '┼' => {
            draw_horiz(buf, 0, cw, mid_y);
            draw_vert(buf, mid_x, 0, ch);
        }
        _ => {}
    }
}
