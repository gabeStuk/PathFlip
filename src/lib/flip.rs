pub const FIELD_Y: f64 = 8.07;
pub const FIELD_X: f64 = 16.54;
pub trait Flippable {
    fn flip_alliance(&mut self);
    fn flip_same_alliance(&mut self);

    fn flip(&mut self, same_alliance: bool) {
        if same_alliance {
            self.flip_same_alliance();
        } else {
            self.flip_alliance();
        }
    }
}

pub fn flip_xaxis(x: f64, y: f64) -> [f64; 2] {
    [x, FIELD_Y - y]
}
pub fn flip_yaxis(x: f64, y: f64) -> [f64; 2] {
    [FIELD_X - x, y]
}
