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

fn spiral_subdivide(bounds: RECT, n: usize) -> Vec<RECT> {
    let mut divisions = vec![bounds];
    for i in 1..n {
        let d = divisions.pop().unwrap();
        let new_d = subdivide(d, i % 2 != 0);
        divisions.extend(new_d);
    }
    divisions
}

fn monocle(bounds: RECT, n: usize) -> Vec<RECT> {
    vec![bounds; n]
}

fn columns(bounds: RECT, n: usize) -> Vec<RECT> {
    let column_width = (bounds.right - bounds.left) / n as i32;
    let mut divisions: Vec<RECT> = vec![];
    for i in 0..n {
        let division = RECT {
            top: bounds.top,
            bottom: bounds.bottom,
            left: i as i32 * column_width,
            right: i as i32 * column_width + column_width,
        };
        divisions.push(division);
    }
    divisions
}

pub enum Arrange {
    Dwindle,
    Monocle,
    Columns,
}

impl Arrange {
    pub fn arrange(&self, bounds: RECT, n: usize) -> Vec<RECT> {
        match self {
            Arrange::Dwindle => spiral_subdivide(bounds, n),
            Arrange::Monocle => monocle(bounds, n),
            Arrange::Columns => columns(bounds, n),
        }
    }
}
