fn subdivide(bounds: (i32, i32, i32, i32), vertical: bool) -> Vec<(i32, i32, i32, i32)> {
    let (bx, by, bw, bh) = bounds;
    if vertical {
        vec![(bx, by, bw / 2, bh), (bx + bw / 2, by, bw / 2, bh)]
    } else {
        vec![(bx, by, bw, bh / 2), (bx, by + bh / 2, bw, bh / 2)]
    }
}

pub fn spiral_subdivide(bounds: (i32, i32, i32, i32), n: usize) -> Vec<(i32, i32, i32, i32)> {
    let mut divisions = vec![bounds];
    for i in 1..n {
        let d = divisions.pop().unwrap();
        let new_d = subdivide(d, i % 2 != 0);
        divisions.extend(new_d);
    }
    divisions
}
