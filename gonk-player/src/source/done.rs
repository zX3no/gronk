use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use crate::{conversions::Sample, source::Source};

/// When the inner source is empty this decrements an `AtomicUsize`.
#[derive(Debug, Clone)]
pub struct Done<I> {
    input: I,
    signal: Arc<AtomicUsize>,
    signal_sent: bool,
}

impl<I> Done<I> {
    #[inline]
    pub fn new(input: I, signal: Arc<AtomicUsize>) -> Done<I> {
        Done {
            input,
            signal,
            signal_sent: false,
        }
    }
}

impl<I: Source> Iterator for Done<I>
where
    I: Source,
    I::Item: Sample,
{
    type Item = I::Item;

    #[inline]
    fn next(&mut self) -> Option<I::Item> {
        let next = self.input.next();
        if !self.signal_sent && next.is_none() {
            self.signal.fetch_sub(1, Ordering::Relaxed);
            self.signal_sent = true;
        }
        next
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.input.size_hint()
    }
}

impl<I> Source for Done<I>
where
    I: Source,
    I::Item: Sample,
{
    #[inline]
    fn current_frame_len(&self) -> Option<usize> {
        self.input.current_frame_len()
    }

    #[inline]
    fn channels(&self) -> u16 {
        self.input.channels()
    }

    #[inline]
    fn sample_rate(&self) -> u32 {
        self.input.sample_rate()
    }

    #[inline]
    fn total_duration(&self) -> Option<Duration> {
        self.input.total_duration()
    }

    #[inline]
    fn elapsed(&mut self) -> Duration {
        self.input.elapsed()
    }
    fn seek(&mut self, time: Duration) -> Option<Duration> {
        self.input.seek(time)
    }
}
