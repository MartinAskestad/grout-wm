use windows::Win32::Foundation::RECT;

fn dwindle(bounds: RECT, n: usize) -> Vec<RECT> {
    (1..n).fold(vec![bounds], |acc, v| {
        let mut my_acc = acc;
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
    (0..n)
        .map(|i| RECT {
            left: i as i32 * column_width,
            top: bounds.top,
            right: i as i32 * column_width + column_width,
            bottom: bounds.bottom,
        })
        .collect()
}

fn focus(bounds: RECT, n: usize) -> Vec<RECT> {
    let lhs: Vec<_> = (0..n)
        .skip(1)
        .filter(|&x| x % 2 == 0)
        .map(|n| n as i32)
        .collect();
    let rhs: Vec<_> = (0..n)
        .skip(1)
        .filter(|&x| x % 2 != 0)
        .map(|n| n as i32)
        .collect();
    (0..n)
        .map(|n| n as i32)
        .enumerate()
        .map(|(idx, val)| match (n, idx, val) {
            (1, _, _) => bounds,
            (2, 0, _) => RECT {
                left: bounds.left,
                top: bounds.top,
                right: bounds.right - (bounds.right / 4),
                bottom: bounds.bottom,
            },
            (2, 1, _) => RECT {
                left: bounds.right - (bounds.right / 4),
                top: bounds.top,
                right: bounds.right,
                bottom: bounds.bottom,
            },
            (_, 0, _) => RECT {
                left: bounds.left + (bounds.right / 4),
                top: bounds.top,
                right: bounds.right - (bounds.right / 4),
                bottom: bounds.bottom,
            },
            (_, _, v) if v % 2 != 0 => RECT {
                left: bounds.right - (bounds.right / 4),
                top: (bounds.bottom / rhs.len() as i32)
                    * rhs.iter().position(|&x| x == v).unwrap() as i32,
                right: bounds.right,
                bottom: (bounds.bottom / rhs.len() as i32)
                    * rhs.iter().position(|&x| x == v).unwrap() as i32
                    + bounds.bottom / rhs.len() as i32,
            },
            (_, _, v) if v % 2 == 0 => RECT {
                left: bounds.left,
                top: (bounds.bottom / lhs.len() as i32)
                    * lhs.iter().position(|&x| x == v).unwrap() as i32,
                right: bounds.left + (bounds.right / 4),
                bottom: (bounds.bottom / lhs.len() as i32)
                    * lhs.iter().position(|&x| x == v).unwrap() as i32
                    + bounds.bottom / lhs.len() as i32,
            },
            _ => Default::default(),
        })
        .collect()
}

pub enum Layouts {
    Dwindle,
    Monocle,
    Columns,
    Focus,
}

impl Layouts {
    pub fn arrange(&self, bounds: RECT, n: usize) -> Vec<RECT> {
        match self {
            Layouts::Dwindle => dwindle(bounds, n),
            Layouts::Monocle => monocle(bounds, n),
            Layouts::Columns => columns(bounds, n),
            Layouts::Focus => focus(bounds, n),
        }
    }
}
