use lazycell::LazyCell;

#[derive(PartialEq)]
#[cfg_attr(test, derive(Debug))]
pub struct APA102DataFrame(u8, u8, u8);

impl APA102DataFrame {
    #[inline]
    fn start_frame_spi_data() -> [u8; 4] {
        [0x00; 4]
    }

    #[inline]
    fn end_frame_spi_data() -> [u8; 4] {
        [0xff; 4]
    }

    fn led_frame(data: u32) -> Self {
        let [_, r, g, b] = data.to_be_bytes();
        APA102DataFrame(r, g, b)
    }

    fn get_spi_data(&self) -> [u8; 4] {
        let APA102DataFrame(r, g, b) = self;
        [0xff, *b, *g, *r]
    }
}

pub struct LEDStrip<const N: usize> {
    data: [APA102DataFrame; N],
    spi_data: LazyCell<Vec<u8>>,
}

impl<const N: usize> LEDStrip<N> {
    pub fn new() -> Self {
        LEDStrip::new_with_data([0; N])
    }

    pub fn new_with_data(data: [u32; N]) -> Self {
        assert!(N > 0, "LEDStrip must have at least one LED");

        Self {
            data: data.map(APA102DataFrame::led_frame),
            spi_data: LazyCell::new(),
        }
    }

    pub fn get_spi_data(&self) -> &Vec<u8> {
        if !self.spi_data.filled() {
            let num_end_frames = (N + 1) / 2;
            let mut spi_data = Vec::with_capacity(N + num_end_frames + 1);
            spi_data.extend(APA102DataFrame::start_frame_spi_data());

            for frame in self.data.iter() {
                spi_data.extend(frame.get_spi_data());
            }

            for _ in 0..num_end_frames {
                spi_data.extend(APA102DataFrame::end_frame_spi_data());
            }

            self.spi_data.fill(spi_data).ok();
        }

        self.spi_data.borrow().unwrap()
    }

    pub fn get_led(&self, index: usize) -> (u8, u8, u8) {
        assert!(index < N, "index out of bounds");
        let APA102DataFrame(r, g, b) = self.data[index];
        (r, g, b)
    }

    pub fn set_led(&mut self, index: usize, color: u32) {
        assert!(index < N, "index out of bounds");

        self.data[index] = APA102DataFrame::led_frame(color);
        if self.spi_data.filled() {
            self.spi_data = LazyCell::new();
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::led::{APA102DataFrame, LEDStrip};

    #[test]
    fn it_builds_grayscale_frames() {
        let black = APA102DataFrame::led_frame(0x000000);
        assert_eq!(black, APA102DataFrame(0, 0, 0));

        let white = APA102DataFrame::led_frame(0xffffff);
        assert_eq!(white, APA102DataFrame(255, 255, 255));
    }

    #[test]
    fn it_builds_color_frames() {
        let red = APA102DataFrame::led_frame(0xff0000);
        assert_eq!(red, APA102DataFrame(255, 0, 0));

        let green = APA102DataFrame::led_frame(0x00ff00);
        assert_eq!(green, APA102DataFrame(0, 255, 0));

        let blue = APA102DataFrame::led_frame(0x0000ff);
        assert_eq!(blue, APA102DataFrame(0, 0, 255));

        let color = APA102DataFrame::led_frame(0x4b8040);
        assert_eq!(color, APA102DataFrame(75, 128, 64));
    }

    #[test]
    #[should_panic(expected = "LEDStrip must have at least one LED")]
    fn it_throws_when_building_an_empty_led_strip() {
        let _led_strip = LEDStrip::<0>::new();
    }

    #[test]
    fn it_makes_frames_for_a_single_led_strip() {
        let led_strip = LEDStrip::new_with_data([0x4b8040]);
        assert_eq!(led_strip.data, [APA102DataFrame(75, 128, 64)]);
        assert_eq!(
            led_strip.get_spi_data(),
            &[
                0x00, 0x00, 0x00, 0x00, // Start frame
                0xff, 0x40, 0x80, 0x4b, // Data frame
                0xff, 0xff, 0xff, 0xff, // End frame
            ]
        );
    }

    #[test]
    fn it_makes_frames_for_an_led_strip() {
        let led_strip = LEDStrip::new_with_data([0xff0000, 0x00ff00, 0x0000ff, 0x4b8040]);
        assert_eq!(
            led_strip.data,
            [
                APA102DataFrame(255, 0, 0),
                APA102DataFrame(0, 255, 0),
                APA102DataFrame(0, 0, 255),
                APA102DataFrame(75, 128, 64),
            ]
        );
        assert_eq!(
            led_strip.get_spi_data(),
            &[
                0x00, 0x00, 0x00, 0x00, // Start frame
                0xff, 0x00, 0x00, 0xff, // Data frame
                0xff, 0x00, 0xff, 0x00, // Data frame
                0xff, 0xff, 0x00, 0x00, // Data frame
                0xff, 0x40, 0x80, 0x4b, // Data frame
                0xff, 0xff, 0xff, 0xff, // End frame
                0xff, 0xff, 0xff, 0xff, // End frame
            ]
        );
    }

    #[test]
    fn it_gets_rgb_values_of_individual_leds() {
        let led_strip = LEDStrip::new_with_data([0xff0000, 0x00ff00, 0x0000ff, 0x4b8040]);
        assert_eq!(led_strip.get_led(1), (0, 255, 0));
        assert_eq!(led_strip.get_led(3), (75, 128, 64));
    }

    #[test]
    fn it_sets_an_led() {
        let mut led_strip = LEDStrip::new_with_data([0xff0000, 0x00ff00, 0x0000ff, 0x4b8040]);
        assert_eq!(
            led_strip.data,
            [
                APA102DataFrame(255, 0, 0),
                APA102DataFrame(0, 255, 0),
                APA102DataFrame(0, 0, 255),
                APA102DataFrame(75, 128, 64),
            ]
        );
        assert_eq!(
            led_strip.get_spi_data(),
            &[
                0x00, 0x00, 0x00, 0x00, // Start frame
                0xff, 0x00, 0x00, 0xff, // Data frame
                0xff, 0x00, 0xff, 0x00, // Data frame
                0xff, 0xff, 0x00, 0x00, // Data frame
                0xff, 0x40, 0x80, 0x4b, // Data frame
                0xff, 0xff, 0xff, 0xff, // End frame
                0xff, 0xff, 0xff, 0xff, // End frame
            ]
        );

        led_strip.set_led(2, 0xf329b2);

        assert_eq!(
            led_strip.data,
            [
                APA102DataFrame(255, 0, 0),
                APA102DataFrame(0, 255, 0),
                APA102DataFrame(243, 41, 178),
                APA102DataFrame(75, 128, 64),
            ]
        );
        assert_eq!(
            led_strip.get_spi_data(),
            &[
                0x00, 0x00, 0x00, 0x00, // Start frame
                0xff, 0x00, 0x00, 0xff, // Data frame
                0xff, 0x00, 0xff, 0x00, // Data frame
                0xff, 0xb2, 0x29, 0xf3, // Data frame
                0xff, 0x40, 0x80, 0x4b, // Data frame
                0xff, 0xff, 0xff, 0xff, // End frame
                0xff, 0xff, 0xff, 0xff, // End frame
            ]
        );
    }
}
