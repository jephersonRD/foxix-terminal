use std::collections::VecDeque;

pub struct RingBuffer<T> {
    buffer: VecDeque<T>,
    capacity: usize,
}

impl<T: Clone> RingBuffer<T> {
    pub fn new(capacity: usize) -> Self {
        Self {
            buffer: VecDeque::with_capacity(capacity),
            capacity,
        }
    }

    pub fn push(&mut self, item: T) {
        if self.buffer.len() >= self.capacity {
            self.buffer.pop_front();
        }
        self.buffer.push_back(item);
    }

    pub fn get(&self, index: usize) -> Option<&T> {
        self.buffer.get(index)
    }

    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }

    pub fn clear(&mut self) {
        self.buffer.clear();
    }

    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.buffer.iter()
    }

    pub fn drain_from_back(&mut self, count: usize) -> Vec<T> {
        let count = count.min(self.buffer.len());
        self.buffer.drain(self.buffer.len() - count..).collect()
    }

    pub fn drain_from_front(&mut self, count: usize) -> Vec<T> {
        let count = count.min(self.buffer.len());
        self.buffer.drain(0..count).collect()
    }
}
