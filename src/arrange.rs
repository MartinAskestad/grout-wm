use windows::Win32::Foundation::RECT;

fn subdivide(bounds: RECT, vertical: bool) -> Vec<RECT> {
    let RECT {
        left,
        top,
        right,
        bottom,
    } = bounds;
    if vertical {
        let mid_x = left + (right - left) / 2;
        vec![
            RECT {
                left,
                top,
                right: mid_x,
                bottom,
            },
            RECT {
                left: mid_x,
                top,
                right,
                bottom,
            },
        ]
    } else {
        let mid_y = top + (bottom - top) / 2;
        vec![
            RECT {
                left,
                top,
                right,
                bottom: mid_y,
            },
            RECT {
                left,
                top: mid_y,
                right,
                bottom,
            },
        ]
    }
}

pub fn spiral_subdivide(bounds: RECT, n: usize) -> Vec<RECT> {
    let mut divisions = vec![bounds];
    for i in 1..n {
        let d = divisions.pop().unwrap();
        let new_d = subdivide(d, i % 2 != 0);
        divisions.extend(new_d);
    }
    divisions
}
