use gonk_core::profile;
use log::*;
use std::{
    ptr,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Condvar, Mutex,
    },
};

//TODO: Add logging to Condvar. Will help with debugging.
#[derive(Debug)]
pub struct Rb<T: Default + Clone> {
    pub buf: Vec<T>,
    pub len: usize, // Note that len-1 is the actual capacity.
    pub write: AtomicUsize,
    pub read: AtomicUsize,

    pub block: Condvar,
    pub is_locked: Mutex<bool>,
}

impl<T: Default + Clone> Rb<T> {
    pub fn new(len: usize) -> Self {
        Self {
            buf: vec![Default::default(); len + 1],
            len: len + 1,
            write: AtomicUsize::new(0),
            read: AtomicUsize::new(0),
            block: Condvar::new(),
            is_locked: Mutex::new(false),
        }
    }

    pub fn push_back(&mut self, value: T) {
        profile!();
        let write = self.write.load(Ordering::SeqCst);
        let read = self.read.load(Ordering::SeqCst);

        if (write + 1) % self.len == read {
            assert!(read != write);

            //Wait for read to free up space.
            {
                let mut is_locked = self.is_locked.lock().unwrap();
                *is_locked = true;
                info!("Locking thread. locked: {}", *is_locked);
                is_locked = self.block.wait(is_locked).unwrap();
            }

            //Try again.
            return self.push_back(value);
        }

        self.write(write, value);
        self.write.store((write + 1) % self.len, Ordering::SeqCst);
    }

    pub fn pop_front(&mut self) -> Option<T> {
        profile!();
        let write = self.write.load(Ordering::SeqCst);
        let read = self.read.load(Ordering::SeqCst);

        if read == write {
            None
        } else {
            //Don't allow write before reading.
            let item = self.read(read);
            self.read.store((read + 1) % self.len, Ordering::SeqCst);

            //Notify the pushing thread to wake up.
            {
                let mut is_locked = self.is_locked.lock().unwrap();
                if *is_locked {
                    *is_locked = false;
                    info!("Unlocking thread. locked: {}", *is_locked);
                    self.block.notify_all();
                }
            }

            Some(item)
        }
    }

    /// Writes an element into the buffer, moving it.
    #[inline]
    fn write(&mut self, off: usize, value: T) {
        unsafe {
            ptr::write(self.buf.as_mut_ptr().add(off), value);
        }
    }

    /// Read an element without moving it.
    #[inline]
    fn read(&mut self, off: usize) -> T {
        unsafe { ptr::read(self.buf.as_mut_ptr().add(off)) }
    }
}

unsafe impl<T: Default + Clone> Send for Rb<T> {}
unsafe impl<T: Default + Clone> Sync for Rb<T> {}
