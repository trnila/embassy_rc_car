type Color = (u8, u8, u8);

pub struct ColorTransition<'a> {
    colors: &'a [Color],
    step: u32,
    index: usize,
    total_steps: u32,
}

impl<'a> ColorTransition<'a> {
    pub fn new(colors: &'a [Color]) -> Self {
        Self {
            colors,
            step: 0,
            index: 0,
            total_steps: 20,
        }
    }

    pub fn next(&mut self) -> Color {
        let color = interpolate_color(
            self.colors[self.index],
            self.colors[(self.index + 1) % self.colors.len()],
            self.step,
            self.total_steps,
        );
        self.step += 1;
        if self.step >= self.total_steps {
            self.step = 0;
            self.index = (self.index + 1) % self.colors.len();
        }
        color
    }
}

fn interpolate_color(
    start: (u8, u8, u8),
    end: (u8, u8, u8),
    step: u32,
    total_steps: u32,
) -> (u8, u8, u8) {
    let (r1, g1, b1) = start;
    let (r2, g2, b2) = end;

    let r = r1 as i32 + ((r2 as i32 - r1 as i32) * step as i32) / total_steps as i32;
    let g = g1 as i32 + ((g2 as i32 - g1 as i32) * step as i32) / total_steps as i32;
    let b = b1 as i32 + ((b2 as i32 - b1 as i32) * step as i32) / total_steps as i32;

    (r as u8, g as u8, b as u8)
}
