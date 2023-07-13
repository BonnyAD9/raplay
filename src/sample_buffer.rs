#[non_exhaustive]
pub enum SampleBufferMut<'a> {
    I8(&'a mut [i8]),
    I16(&'a mut [i16]),
    I32(&'a mut [i32]),
    I64(&'a mut [i64]),
    U8(&'a mut [u8]),
    U16(&'a mut [u16]),
    U32(&'a mut [u32]),
    U64(&'a mut [u64]),
    F32(&'a mut [f32]),
    F64(&'a mut [f64]),
}

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
