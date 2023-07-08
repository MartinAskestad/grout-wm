use crate::rect::Rect;

fn subdivide(bounds: Rect, vertical: bool) -> Vec<Rect> {
    if vertical {
        vec![
            Rect::new(bounds.left, bounds.top, bounds.width / 2, bounds.height),
            Rect::new(
                bounds.left + bounds.width / 2,
                bounds.top,
                bounds.width / 2,
                bounds.height,
            ),
        ]
    } else {
        vec![
            Rect::new(bounds.left, bounds.top, bounds.width, bounds.height / 2),
            Rect::new(
                bounds.left,
                bounds.top + bounds.height / 2,
                bounds.width,
                bounds.height / 2,
            ),
        ]
    }
}

pub fn spiral_subdivide(bounds: Rect, n: usize) -> Vec<Rect> {
    let mut divisions = vec![bounds];
    for i in 1..n {
        let d = divisions.pop().unwrap();
        let new_d = subdivide(d, i % 2 != 0);
        divisions.extend(new_d);
    }
    divisions
}
