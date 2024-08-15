use std::ptr::null_mut;

use field_count::FieldCount;
use serde::{Deserialize, Serialize};
use serde_bytes::Bytes;
use tarantool::tuple::{Encode, TupleBuffer};

mod ffi {
    use super::BoxTuple;
    extern "C" {
        pub fn box_tuple_unref(tuple: *mut BoxTuple);

        pub fn box_tuple_bsize(tuple: *const BoxTuple) -> usize;
    }
}

/// Tarantool crate resolves BoxTuple in the right way
/// only if picodata feature is used.
/// So we use own type.
#[repr(C, packed)]
pub struct BoxTuple {
    refs: u8,
    _flags: u8,
    format_id: u16,
    data_offset: u16,
    bsize: u32,
}

/// Tarantool crate does not expose raw tuple pointer
/// so we reinterpret returned tuple as own type.
#[derive(Debug)]
struct Tuple {
    ptr: *mut BoxTuple,
}

impl Tuple {
    fn bsize(&self) -> usize {
        // Safety: `Tuple` is created only in `raw_tuple_to_entry`.
        // Check it.
        unsafe { ffi::box_tuple_bsize(self.ptr) }
    }
}

impl Drop for Tuple {
    fn drop(&mut self) {
        if !self.ptr.is_null() {
            // Safety: `Tuple` with non null ptr is created only
            // in `raw_tuple_to_entry`.
            // Check it.
            unsafe { ffi::box_tuple_unref(self.ptr) }
        }
    }
}

impl Default for Tuple {
    fn default() -> Self {
        Self { ptr: null_mut() }
    }
}

#[derive(Debug, Deserialize, Serialize, FieldCount)]
pub struct Entry<'a> {
    pub id: usize,
    #[serde(borrow)]
    pub data: &'a Bytes,
    #[serde(skip)]
    // To decrease tuple reference counter at the life end.
    _raw_tuple: Tuple,
}

impl<'a> Entry<'a> {
    pub fn new(id: usize, data: &'a Bytes) -> Self {
        Self {
            id: id,
            data: data,
            _raw_tuple: Default::default(),
        }
    }
}

impl<'a> Encode for Entry<'a> {}

/// Abstraction to work with `bindata` space.
pub struct BinSpace {
    space: tarantool::space::Space,
}

impl BinSpace {
    /// Assumed that space was created in init.lua.
    pub fn try_new() -> Option<Self> {
        let space = tarantool::space::Space::find("bindata")?;
        Some(Self { space: space })
    }

    pub fn put(&self, entry: Entry) {
        self.space.put(&entry).unwrap();
    }

    pub fn update(&self, id: usize, ops: &[TupleBuffer]) {
        self.space.update(&(id,), ops).unwrap();
    }

    pub fn get(&self, id: usize) -> Option<Entry> {
        // Don't process tarantool array.
        let tuple = self.space.get(&(id,)).unwrap()?;

        // Safety: Since tuple exists, pointer is not null and layout is correct.
        // `get` has increased tuple reference count, hence this tuple will
        // be valid until this struct is not dropped.
        unsafe { raw_tuple_to_entry(tuple) }
    }
}

/// Safety: for now works only for non-compact tuples.
/// About compact tuples:
/// https://github.com/tarantool/tarantool/blob/1b0cc057f3b06138e61a47be4309200186691200/src/box/tuple.h#L394-L406
unsafe fn raw_tuple_to_entry<'a, T>(tuple: tarantool::tuple::Tuple) -> T
where
    T: Deserialize<'a>,
{
    let tuple: Tuple = unsafe { std::mem::transmute(tuple) };
    let data_ptr = unsafe {
        let box_tuple = tuple
            .ptr
            .as_ref()
            .expect("tuple ptr is not null as tuple exists");
        // TODO: process compact tuples in the right way.
        let offset = box_tuple.data_offset;

        let msgpack_data_begin = (tuple.ptr as *mut u8).add(offset as usize);
        let msgpack_data_length = tuple.bsize();
        std::slice::from_raw_parts(msgpack_data_begin, msgpack_data_length)
    };

    rmp_serde::from_slice::<T>(data_ptr).unwrap()
}
