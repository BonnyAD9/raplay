/// Buffer of samples, this is enum that contains the possible types
/// of samples in a buffer
#[non_exhaustive]
pub enum SampleBufferMut<'a> {
    // documentation is copied from sample_formats.rs in cpal
    /// `i8` with a valid range of 'u8::MIN..=u8::MAX' with `0` being the
    /// origin
    I8(&'a mut [i8]),
    /// `i16` with a valid range of 'u16::MIN..=u16::MAX' with `0` being the
    /// origin
    I16(&'a mut [i16]),
    /// `i32` with a valid range of 'u32::MIN..=u32::MAX' with `0` being the
    /// origin
    I32(&'a mut [i32]),
    /// `i64` with a valid range of 'u64::MIN..=u64::MAX' with `0` being the
    /// origin
    I64(&'a mut [i64]),
    /// `u8` with a valid range of 'u8::MIN..=u8::MAX' with `1 << 7 == 128`
    /// being the origin
    U8(&'a mut [u8]),
    /// `u16` with a valid range of 'u16::MIN..=u16::MAX' with
    /// `1 << 15 == 32768` being the origin
    U16(&'a mut [u16]),
    /// `u32` with a valid range of 'u32::MIN..=u32::MAX' with `1 << 31` being
    /// the origin
    U32(&'a mut [u32]),
    /// `u64` with a valid range of 'u64::MIN..=u64::MAX' with `1 << 63` being
    /// the origin
    U64(&'a mut [u64]),
    /// `f32` with a valid range of `-1.0..1.0` with `0.0` being the origin
    F32(&'a mut [f32]),
    /// `f64` with a valid range of -1.0..1.0 with 0.0 being the origin
    F64(&'a mut [f64]),
}

/// Does operation on the variant of the buffer
#[macro_export]
macro_rules! operate_samples {
    ($buf:expr, $id:ident, $op:expr) => {{
        use $crate::sample_buffer::SampleBufferMut;
        match $buf {
            SampleBufferMut::I8($id) => $op,
            SampleBufferMut::I16($id) => $op,
            SampleBufferMut::I32($id) => $op,
            SampleBufferMut::I64($id) => $op,
            SampleBufferMut::U8($id) => $op,
            SampleBufferMut::U16($id) => $op,
            SampleBufferMut::U32($id) => $op,
            SampleBufferMut::U64($id) => $op,
            SampleBufferMut::F32($id) => $op,
            SampleBufferMut::F64($id) => $op,
        }
    }};
}

// I wasn't able to make the following macros into functions because of some
// lifetime requirements.

/// Creates slice from the buffer
#[macro_export]
macro_rules! slice_sbuf {
    ($buf:expr, $range:expr) => {{
        use $crate::sample_buffer::SampleBufferMut;
        match $buf {
            SampleBufferMut::I8(d) => SampleBufferMut::I8(&mut d[$range]),
            SampleBufferMut::I16(d) => SampleBufferMut::I16(&mut d[$range]),
            SampleBufferMut::I32(d) => SampleBufferMut::I32(&mut d[$range]),
            SampleBufferMut::I64(d) => SampleBufferMut::I64(&mut d[$range]),
            SampleBufferMut::U8(d) => SampleBufferMut::U8(&mut d[$range]),
            SampleBufferMut::U16(d) => SampleBufferMut::U16(&mut d[$range]),
            SampleBufferMut::U32(d) => SampleBufferMut::U32(&mut d[$range]),
            SampleBufferMut::U64(d) => SampleBufferMut::U64(&mut d[$range]),
            SampleBufferMut::F32(d) => SampleBufferMut::F32(&mut d[$range]),
            SampleBufferMut::F64(d) => SampleBufferMut::F64(&mut d[$range]),
        }
    }};
}

/// Writes silence into the buffer
#[macro_export]
macro_rules! silence_sbuf {
    ($buf:expr) => {
        operate_samples!($buf, b, write_silence(b))
    };
}

impl<'a> SampleBufferMut<'a> {
    /// Gets the number of items in the buffer
    pub fn len(&self) -> usize {
        operate_samples!(self, b, b.len())
    }

    /// Checks if the buffer is empty
    pub fn is_empty(&self) -> bool {
        operate_samples!(self, b, b.is_empty())
    }
}

/// Writes silence to the buffer
pub fn write_silence<T: cpal::Sample>(data: &mut [T]) {
    data.fill(T::EQUILIBRIUM);
}
