pub const XMIN: u16 = 0;
pub const YMIN: u16 = 0;
pub const XMAX: u16 = 79;
pub const YMAX: u16 = 24;
pub const GAME_TICK_MILLIS: u64 = 10;
pub const FIELD_SIZE: u16 = XMAX * YMAX;
pub const PIXEL_EMPTY: u8 = b' ';

pub struct Game {
    pub xpos: u16,
    pub ypos: u16,
    pub size: u16,
    pub pixel: u8,
    pub xmov: f32,
    pub ymov: f32,
    pub xf32: f32,
    pub yf32: f32,
}

impl Game {
    pub fn get_ymin(self: &Self) -> u16 {
        self.ypos
    }

    pub fn get_ymax(self: &Self) -> u16 {
        self.ypos + self.size
    }
}

impl Game {
    pub fn new(x: u16, y: u16, size: u16, pixel: u8) -> Self {
        Self {
            xpos: x,
            ypos: y,
            size,
            pixel,
            xmov: 0.0,
            ymov: 0.0,
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
            field_data: [PIXEL_EMPTY; FIELD_SIZE as usize],
        }
    }
    pub fn clear(self: &mut Self) {
        for i in 0..self.field_data.len() {
            let x: u16 = i as u16 % XMAX;
            let y: u16 = i as u16 / XMAX;
            let c: u8;

            if y == YMIN || y == YMAX - 1 {
                c = b'-';
            } else if x == (XMAX - XMIN) / 2 {
                c = b'\'';
            } else if x == XMIN || x == XMAX - 1 {
                c = b'|';
            } else {
                c = PIXEL_EMPTY;
            }

            self.field_data[i as usize] = c;
        }
    }

    pub fn get_idx(self: &Self, x: &u16, y: &u16) -> usize {
        (x + y * XMAX) as usize
    }

    pub fn draw(self: &mut Self, game: &Game) {
        let x = game.xpos;
        for y in game.get_ymin()..game.get_ymax() {
            self.field_data[self.get_idx(&x, &y)] = game.pixel;
        }
    }

    pub fn write(self: &mut Self, x: u16, y: u16, text: &str) {
        let i = self.get_idx(&x, &y);
        text.as_bytes().iter().enumerate().for_each(|(j, c)| {
            self.field_data[i + j] = *c;
        });
    }
}
