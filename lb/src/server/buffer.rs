use std::mem::ManuallyDrop;

pub struct Buffers {
    pub buffer_base: *mut u8,
    pub buffer_segment_count: u16,
    pub buffer_segment_len: usize,
}

impl Buffers {
    pub fn new(buffer_segment_count: u16, buffer_segment_len: usize) -> Self {
        let buffer = Vec::with_capacity(buffer_segment_count as usize * buffer_segment_len);

        Self {
            buffer_base: ManuallyDrop::new(buffer).as_mut_ptr(),
            buffer_segment_len,
            buffer_segment_count,
        }
    }

    pub fn get_segment(&self, idx: u16) -> &[u8] {
        assert!(idx < self.buffer_segment_count);
        let idx = idx as usize;
        let offset = idx * self.buffer_segment_len;
        let ptr = self.buffer_base.wrapping_add(offset);
        unsafe { std::slice::from_raw_parts(ptr, self.buffer_segment_len) }
    }
}
