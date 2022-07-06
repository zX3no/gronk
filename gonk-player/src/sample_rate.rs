//https://github.com/RustAudio/rodio/blob/master/src/conversions/sample_rate.rs
use std::{mem, vec::IntoIter};

#[inline]
const fn gcd(a: u32, b: u32) -> u32 {
    if b == 0 {
        a
    } else {
        gcd(b, a % b)
    }
}

#[inline]
const fn lerp(a: f32, b: f32, t: f32) -> f32 {
    return a + t * (b - a);
}

/// Iterator that converts from a certain sample rate to another.
pub struct SampleRateConverter {
    /// The iterator that gives us samples.
    input: IntoIter<f32>,
    /// We convert chunks of `from` samples into chunks of `to` samples.
    from: u32,
    /// We convert chunks of `from` samples into chunks of `to` samples.
    to: u32,
    /// One sample per channel, extracted from `input`.
    current_frame: Vec<f32>,
    /// Position of `current_sample` modulo `from`.
    current_frame_pos_in_chunk: u32,
    /// The samples right after `current_sample` (one per channel), extracted from `input`.
    next_frame: Vec<f32>,
    /// The position of the next sample that the iterator should return, modulo `to`.
    /// This counter is incremented (modulo `to`) every time the iterator is called.
    next_output_frame_pos_in_chunk: u32,
    /// The buffer containing the samples waiting to be output.
    output_buffer: Option<f32>,
}

impl SampleRateConverter {
    pub fn new(mut input: IntoIter<f32>, from_rate: u32, to_rate: u32) -> SampleRateConverter {
        assert!(from_rate >= 1);
        assert!(to_rate >= 1);

        // finding greatest common divisor
        let gcd = gcd(from_rate, to_rate);

        let (first_samples, next_samples) = if from_rate == to_rate {
            // if `from` == `to` == 1, then we just pass through
            debug_assert_eq!(from_rate, gcd);
            (Vec::new(), Vec::new())
        } else {
            let first = vec![input.next().unwrap(), input.next().unwrap()];
            let next = vec![input.next().unwrap(), input.next().unwrap()];
            (first, next)
        };

        SampleRateConverter {
            input,
            from: from_rate / gcd,
            to: to_rate / gcd,
            current_frame_pos_in_chunk: 0,
            next_output_frame_pos_in_chunk: 0,
            current_frame: first_samples,
            next_frame: next_samples,
            output_buffer: None,
        }
    }

    fn next_input_frame(&mut self) {
        self.current_frame_pos_in_chunk += 1;

        mem::swap(&mut self.current_frame, &mut self.next_frame);
        self.next_frame.clear();
        if let Some(i) = self.input.next() {
            self.next_frame.push(i);
        }
        if let Some(i) = self.input.next() {
            self.next_frame.push(i);
        }
    }

    pub fn update(&mut self, mut input: IntoIter<f32>) {
        let (current_frame, next_frame) = if self.from == self.to {
            (Vec::new(), Vec::new())
        } else {
            let current = vec![input.next().unwrap(), input.next().unwrap()];
            let next = vec![input.next().unwrap(), input.next().unwrap()];
            (current, next)
        };
        self.current_frame = current_frame;
        self.next_frame = next_frame;
        self.input = input;
        self.current_frame_pos_in_chunk = 0;
        self.next_output_frame_pos_in_chunk = 0;
    }

    pub fn next(&mut self) -> Option<f32> {
        // the algorithm below doesn't work if `self.from == self.to`
        if self.from == self.to {
            return self.input.next();
        }

        // Short circuit if there are some samples waiting.
        if let Some(output) = self.output_buffer.take() {
            return Some(output);
        }

        // The frame we are going to return from this function will be a linear interpolation
        // between `self.current_frame` and `self.next_frame`.

        if self.next_output_frame_pos_in_chunk == self.to {
            // If we jump to the next frame, we reset the whole state.
            self.next_output_frame_pos_in_chunk = 0;

            self.next_input_frame();
            while self.current_frame_pos_in_chunk != self.from {
                self.next_input_frame();
            }
            self.current_frame_pos_in_chunk = 0;
        } else {
            // Finding the position of the first sample of the linear interpolation.
            let req_left_sample =
                (self.from * self.next_output_frame_pos_in_chunk / self.to) % self.from;

            // Advancing `self.current_frame`, `self.next_frame` and
            // `self.current_frame_pos_in_chunk` until the latter variable
            // matches `req_left_sample`.
            while self.current_frame_pos_in_chunk != req_left_sample {
                self.next_input_frame();
                debug_assert!(self.current_frame_pos_in_chunk < self.from);
            }
        }

        // Merging `self.current_frame` and `self.next_frame` into `self.output_buffer`.
        // Note that `self.output_buffer` can be truncated if there is not enough data in
        // `self.next_frame`.
        let numerator = (self.from * self.next_output_frame_pos_in_chunk) % self.to;

        // Incrementing the counter for the next iteration.
        self.next_output_frame_pos_in_chunk += 1;

        if self.next_frame.is_empty() {
            if self.current_frame.is_empty() {
                None
            } else {
                let r = Some(self.current_frame.remove(0));
                let current_frame = self.current_frame[0];
                self.output_buffer = Some(current_frame);
                r
            }
        } else {
            let ratio = numerator as f32 / self.to as f32;
            let sample = lerp(self.current_frame[1], self.next_frame[1], ratio);
            self.output_buffer = Some(sample);
            Some(lerp(self.current_frame[0], self.next_frame[0], ratio))
        }
    }
}
