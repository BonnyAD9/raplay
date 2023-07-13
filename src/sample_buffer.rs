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
/// 
/// you need to import SampleBufferMut for this to work
#[macro_export]
macro_rules! operate_samples {
    ($buf:expr, $id:ident, $op:expr) => {
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
    };
}
