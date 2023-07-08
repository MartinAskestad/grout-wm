#[derive(Clone, Copy, Debug)]
pub struct Rect {
    pub left: i32,
    pub top: i32,
    pub width: i32,
    pub height: i32,
}

impl Rect {
    pub fn new(left: i32, top: i32, width: i32, height: i32) -> Self {
        Rect { left, top, width, height }
    }
}

impl std::convert::From<windows::Win32::Foundation::RECT> for Rect {
    fn from(r: windows::Win32::Foundation::RECT) -> Self {
        Rect {
            left: r.left,
            top: r.top,
            width: r.right - r.left,
            height: r.bottom - r.top,
        }
    }
}

impl std::ops::Sub<windows::Win32::Foundation::RECT> for Rect {
    type Output = Rect;
    fn sub(self, b: windows::Win32::Foundation::RECT) -> Rect {
        Rect {
            left: self.left - b.left,
            top: self.top - b.top,
            width: self.width - (b.right - b.left),
            height: self.height - (b.bottom - b.top),
        }
    }
}
