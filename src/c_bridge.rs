use std::ffi::CString;

pub fn create_sized_cstring(len: usize) -> CString {
    let mut buffer: Vec<u8> = Vec::with_capacity(len + 1);
    unsafe { CString::from_vec_unchecked(buffer) }
}
