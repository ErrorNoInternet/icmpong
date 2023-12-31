pub const X_MINIMUM: u16 = 0;
pub const Y_MINIMUM: u16 = 0;
pub const X_MAXIMUM: u16 = 79;
pub const Y_MAXIMUM: u16 = 24;
pub const GAME_TICK_MILLISECONDS: u64 = 15;
pub const FIELD_SIZE: u16 = X_MAXIMUM * Y_MAXIMUM;

pub struct GameObject {
    pub x_position: u16,
    pub y_position: u16,
    pub size: u16,
    pub pixel: u8,
    pub x_movement: f32,
    pub y_movement: f32,
    pub xf32: f32,
    pub yf32: f32,
}

impl GameObject {
    pub fn get_ymin(self: &Self) -> u16 {
        self.y_position
    }

    pub fn get_ymax(self: &Self) -> u16 {
        self.y_position + self.size
    }
}

impl GameObject {
    pub fn new(x: u16, y: u16, size: u16, pixel: u8) -> Self {
        Self {
            x_position: x,
            y_position: y,
            size,
            pixel,
            x_movement: 0.0,
            y_movement: 0.0,
            xf32: x as f32,
            yf32: y as f32,
        }
    }
}

pub struct Field {
    pub field_data: [u8; FIELD_SIZE as usize],
}

impl Field {
    pub fn new() -> Self {
        Self {
            field_data: [b' '; FIELD_SIZE as usize],
        }
    }

    pub fn clear(self: &mut Self) {
        for i in 0..self.field_data.len() {
            let x: u16 = i as u16 % X_MAXIMUM;
            let y: u16 = i as u16 / X_MAXIMUM;
            let c: u8;

            if y == Y_MINIMUM || y == Y_MAXIMUM - 1 {
                c = b'-';
            } else if x == (X_MAXIMUM - X_MINIMUM) / 2 {
                c = b'\'';
            } else if x == X_MINIMUM || x == X_MAXIMUM - 1 {
                c = b'|';
            } else {
                c = b' ';
            }

            self.field_data[i as usize] = c;
        }
    }

    pub fn get_idx(self: &Self, x: &u16, y: &u16) -> usize {
        (x + y * X_MAXIMUM) as usize
    }

    pub fn draw(self: &mut Self, game: &GameObject) {
        let x = game.x_position;
        for y in game.get_ymin()..game.get_ymax() {
            let index = self.get_idx(&x, &y);
            if self.field_data.len() > index {
                self.field_data[index] = game.pixel;
            }
        }
    }

    pub fn write(self: &mut Self, x: u16, y: u16, text: &str) {
        let i = self.get_idx(&x, &y);
        text.as_bytes().iter().enumerate().for_each(|(j, c)| {
            self.field_data[i + j] = *c;
        });
    }
}
