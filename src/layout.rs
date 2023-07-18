use windows::Win32::Foundation::RECT;

fn dwindle(bounds: RECT, n: usize) -> Vec<RECT> {
    (1..n).fold(vec![bounds.clone()], |acc, v| {
        let mut my_acc = acc.clone();
        let bounds = my_acc.pop().unwrap();
        let vertical = v % 2 != 0;
        let RECT {
            left,
            top,
            right,
            bottom,
        } = bounds;
        let rs: Vec<RECT> = if vertical {
            vec![
                RECT {
                    left,
                    top,
                    right: left + (right - left) / 2,
                    bottom,
                },
                RECT {
                    left: left + (right - left) / 2,
                    top,
                    right,
                    bottom,
                },
            ]
        } else {
            vec![
                RECT {
                    left,
                    top,
                    right,
                    bottom: top + (bottom - top) / 2,
                },
                RECT {
                    left,
                    top: top + (bottom - top) / 2,
                    right,
                    bottom,
                },
            ]
        };
        my_acc.extend(rs);
        my_acc
    })
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

pub enum Layouts {
    Dwindle,
    Monocle,
    Columns,
}

impl Layouts {
    pub fn arrange(&self, bounds: RECT, n: usize) -> Vec<RECT> {
        match self {
            Layouts::Dwindle => dwindle(bounds, n),
            Layouts::Monocle => monocle(bounds, n),
            Layouts::Columns => columns(bounds, n),
        }
    }
}
