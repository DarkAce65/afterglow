#[derive(PartialEq)]
#[cfg_attr(test, derive(Debug))]
pub enum APA102DataFrame {
    Start,
    End,
    Led(u8, u8, u8),
}

impl APA102DataFrame {
    fn led_frame(data: u32) -> Self {
        let [_, b, g, r] = data.to_be_bytes();
        APA102DataFrame::Led(r, g, b)
    }
}

impl From<APA102DataFrame> for u32 {
    fn from(frame: APA102DataFrame) -> Self {
        match frame {
            APA102DataFrame::Start => 0x00000000,
            APA102DataFrame::End => 0xffffffff,
            APA102DataFrame::Led(r, g, b) => {
                (255 << 24) + (u32::from(b) << 16) + (u32::from(g) << 8) + u32::from(r)
            }
        }
    }
}

impl From<APA102DataFrame> for [u8; 4] {
    fn from(frame: APA102DataFrame) -> Self {
        match frame {
            APA102DataFrame::Start => [0x00; 4],
            APA102DataFrame::End => [0xff; 4],
            APA102DataFrame::Led(r, g, b) => [0xff, b, g, r],
        }
    }
}

pub struct LEDStrip<const N: usize> {
    pub data: [u32; N],
}

impl<const N: usize> LEDStrip<N> {
    pub fn make_data_frames(&self) -> Vec<APA102DataFrame> {
        if self.data.is_empty() {
            return Vec::new();
        }

        let num_end_frames = (self.data.len() + 1) / 2;
        let mut data_frames = Vec::with_capacity(N + 1 + num_end_frames);

        data_frames.push(APA102DataFrame::Start);

        self.data
            .into_iter()
            .for_each(|d| data_frames.push(APA102DataFrame::led_frame(d)));

        for _ in 0..num_end_frames {
            data_frames.push(APA102DataFrame::End);
        }

        data_frames
    }
}

#[cfg(test)]
mod tests {
    use crate::led::{APA102DataFrame, LEDStrip};

    #[test]
    fn it_converts_a_start_frame() {
        let frame = APA102DataFrame::Start;
        assert_eq!(u32::from(frame), 0x00000000);
    }

    #[test]
    fn it_converts_an_end_frame() {
        let frame = APA102DataFrame::End;
        assert_eq!(u32::from(frame), 0xffffffff);
    }

    #[test]
    fn it_converts_grayscale_frames() {
        let black = APA102DataFrame::Led(0, 0, 0);
        assert_eq!(u32::from(black), 0xff000000);

        let white = APA102DataFrame::Led(255, 255, 255);
        assert_eq!(u32::from(white), 0xffffffff);
    }

    #[test]
    fn it_converts_color_frames() {
        let red = APA102DataFrame::Led(255, 0, 0);
        assert_eq!(u32::from(red), 0xff0000ff);

        let green = APA102DataFrame::Led(0, 255, 0);
        assert_eq!(u32::from(green), 0xff00ff00);

        let blue = APA102DataFrame::Led(0, 0, 255);
        assert_eq!(u32::from(blue), 0xffff0000);

        let color = APA102DataFrame::Led(64, 128, 75);
        assert_eq!(u32::from(color), 0xff4b8040);
    }

    #[test]
    fn it_builds_grayscale_frames() {
        let black = APA102DataFrame::led_frame(0xff000000);
        assert_eq!(black, APA102DataFrame::Led(0, 0, 0));

        let white = APA102DataFrame::led_frame(0xffffffff);
        assert_eq!(white, APA102DataFrame::Led(255, 255, 255));
    }

    #[test]
    fn it_builds_color_frames() {
        let red = APA102DataFrame::led_frame(0xff0000ff);
        assert_eq!(red, APA102DataFrame::Led(255, 0, 0));

        let green = APA102DataFrame::led_frame(0xff00ff00);
        assert_eq!(green, APA102DataFrame::Led(0, 255, 0));

        let blue = APA102DataFrame::led_frame(0xffff0000);
        assert_eq!(blue, APA102DataFrame::Led(0, 0, 255));

        let color = APA102DataFrame::led_frame(0xff4b8040);
        assert_eq!(color, APA102DataFrame::Led(64, 128, 75));
    }

    #[test]
    fn it_makes_frames_for_an_empty_led_strip() {
        let led_strip = LEDStrip { data: [] };
        assert_eq!(led_strip.make_data_frames(), []);
    }

    #[test]
    fn it_makes_frames_for_a_single_led_strip() {
        let led_strip = LEDStrip { data: [0x4b8040] };
        assert_eq!(
            led_strip.make_data_frames(),
            [
                APA102DataFrame::Start,
                APA102DataFrame::Led(64, 128, 75),
                APA102DataFrame::End,
            ]
        );
    }

    #[test]
    fn it_makes_frames_for_an_led_strip() {
        let led_strip = LEDStrip {
            data: [0x0000ff, 0x00ff00, 0xff0000, 0x4b8040],
        };
        assert_eq!(
            led_strip.make_data_frames(),
            [
                APA102DataFrame::Start,
                APA102DataFrame::Led(255, 0, 0),
                APA102DataFrame::Led(0, 255, 0),
                APA102DataFrame::Led(0, 0, 255),
                APA102DataFrame::Led(64, 128, 75),
                APA102DataFrame::End,
                APA102DataFrame::End,
            ]
        );
    }
}
