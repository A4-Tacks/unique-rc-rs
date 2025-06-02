#![doc = include_str!("../README.md")]
#![cfg_attr(not(feature = "std"), no_std)]
#![warn(unsafe_op_in_unsafe_fn)]

extern crate alloc;

#[cfg(feature = "std")]
use std::io;

use alloc::{boxed::Box, rc::Rc, string::String, sync::Arc};
use core::{
    any::Any,
    borrow::{Borrow, BorrowMut},
    cmp,
    error::Error,
    ffi::CStr,
    fmt,
    future::Future,
    hash::{Hash, Hasher},
    iter::FusedIterator,
    ops::{Deref, DerefMut},
    pin::Pin,
    task::{Context, Poll},
};

/// # Safety
/// - The implementation of [`make_mut`] and [`to_unique`]
///   must ensure that `strong_count` are set to 1 and there are no weak references
///
/// [`make_mut`]: #tymethod.make_mut
/// [`to_unique`]: #method.to_unique
pub unsafe trait MakeMut: Sized {
    type T: ?Sized;

    fn make_mut(this: &mut Self) -> &mut Self::T;

    fn to_unique(mut this: Self) -> Self {
        Self::make_mut(&mut this);
        this
    }
}
macro_rules! impl_make_mut {
    ($({$($g:tt)*})? $ty:ty) => {
        unsafe impl$(<$($g)*>)? MakeMut for Rc<$ty> {
            type T = $ty;

            fn make_mut(this: &mut Self) -> &mut Self::T {
                Self::make_mut(this)
            }
        }
        unsafe impl$(<$($g)*>)? MakeMut for Arc<$ty> {
            type T = $ty;

            fn make_mut(this: &mut Self) -> &mut Self::T {
                Self::make_mut(this)
            }
        }
        unsafe impl$(<$($g)*>)? MakeMut for UniqRc<$ty> {
            type T = $ty;

            fn make_mut(this: &mut Self) -> &mut Self::T {
                this
            }
        }
    };
}
impl_make_mut!({T: Clone} T);
impl_make_mut!({T: Clone} [T]);
impl_make_mut!(str);
impl_make_mut!(CStr);

#[cfg(feature = "std")]
impl_make_mut!(std::path::Path);
#[cfg(feature = "std")]
impl_make_mut!(std::ffi::OsStr);

macro_rules! impl_downcast {
    ($UniqRc:ident : $(+ $auto:ident)*) => {
        impl $UniqRc<dyn Any $(+ $auto)* + 'static> {
            /// Like [`Box::downcast`]
            ///
            /// # Errors
            /// - `self.is::<T>() == false`
            pub fn downcast<T>(self) -> Result<$UniqRc<T>, Self>
            where T: Any,
            {
                if self.is::<T>() {
                    let raw: *mut (dyn Any $(+ $auto)*) = Self::into_raw(self);
                    Ok(unsafe { $UniqRc::from_raw_unchecked(raw.cast()) })
                } else {
                    Err(self)
                }
            }
        }
    };
}

macro_rules! impl_rc { ($Rc:ident, $UniqRc:ident) => {

#[doc = concat!("Owned [`", stringify!($Rc), "`], like [`Box`]\n\n")]
#[doc = concat!("No other [`", stringify!($Rc), "`] or `Weak`\n")]
#[repr(transparent)]
#[derive(Eq, Default)]
pub struct $UniqRc<T: ?Sized> {
    rc: $Rc<T>,
}

impl<T: ?Sized + Hash> Hash for $UniqRc<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.rc.hash(state);
    }
}

impl<T: ?Sized + Ord> Ord for $UniqRc<T> {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        self.rc.cmp(&other.rc)
    }
}

impl<T: ?Sized + PartialOrd> PartialOrd for $UniqRc<T> {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        self.rc.partial_cmp(&other.rc)
    }
}

impl<T: ?Sized + PartialEq> PartialEq for $UniqRc<T> {
    fn eq(&self, other: &Self) -> bool {
        self.rc == other.rc
    }
}

impl<T: ?Sized> AsRef<T> for $UniqRc<T> {
    fn as_ref(&self) -> &T {
        self
    }
}

impl<T: ?Sized> AsMut<T> for $UniqRc<T> {
    fn as_mut(&mut self) -> &mut T {
        self
    }
}

impl<T: ?Sized> Borrow<T> for $UniqRc<T> {
    fn borrow(&self) -> &T {
        self
    }
}

impl<T: ?Sized> BorrowMut<T> for $UniqRc<T> {
    fn borrow_mut(&mut self) -> &mut T {
        self
    }
}

impl<T: ?Sized> Clone for $UniqRc<T>
where $Rc<T>: MakeMut<T = T>,
{
    fn clone(&self) -> Self {
        Self::new(MakeMut::to_unique(self.rc.clone()))
    }
}

impl<T: ?Sized + Error> Error for $UniqRc<T> {
    #[allow(deprecated)]
    fn cause(&self) -> Option<&dyn Error> {
        (**self).cause()
    }

    #[allow(deprecated)]
    fn description(&self) -> &str {
        (**self).description()
    }

    fn source(&self) -> Option<&(dyn Error + 'static)> {
        (**self).source()
    }
}

impl<T: ?Sized + fmt::Debug> fmt::Debug for $UniqRc<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.rc.fmt(f)
    }
}

impl<T: ?Sized + fmt::Display> fmt::Display for $UniqRc<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.rc.fmt(f)
    }
}

impl<T: ?Sized + fmt::Pointer> fmt::Pointer for $UniqRc<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.rc.fmt(f)
    }
}

impl<T: ?Sized> Deref for $UniqRc<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        debug_assert_eq!($Rc::strong_count(&self.rc), 1);
        &*self.rc
    }
}

impl<T: ?Sized> DerefMut for $UniqRc<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        debug_assert_eq!($Rc::strong_count(&self.rc), 1);
        $Rc::get_mut(&mut self.rc).unwrap()
    }
}

impl<T: ?Sized, U> FromIterator<U> for $UniqRc<T>
where $Rc<T>: FromIterator<U> + MakeMut<T = T>,
{
    fn from_iter<I: IntoIterator<Item = U>>(iter: I) -> Self {
        let rc = $Rc::from_iter(iter);
        Self::new(rc)
    }
}

impl<T: ?Sized, U> From<U> for $UniqRc<T>
where $Rc<T>: MakeMut<T = T> + From<U>,
{
    fn from(value: U) -> Self {
        Self::new(value.into())
    }
}

impl<T: ?Sized> From<$UniqRc<T>> for Pin<$UniqRc<T>> {
    fn from(this: $UniqRc<T>) -> Self {
        $UniqRc::into_pin(this)
    }
}

impl<T, const N: usize> From<$UniqRc<[T; N]>> for $UniqRc<[T]> {
    fn from(this: $UniqRc<[T; N]>) -> Self {
        let new = $UniqRc::into_raw(this);
        unsafe { Self::from_raw_unchecked(new) }
    }
}

impl Extend<$UniqRc<str>> for String {
    fn extend<I: IntoIterator<Item = $UniqRc<str>>>(&mut self, iter: I) {
        iter.into_iter().for_each(|s| self.push_str(&s))
    }
}

impl FromIterator<$UniqRc<str>> for String {
    fn from_iter<I: IntoIterator<Item = $UniqRc<str>>>(iter: I) -> Self {
        let mut buf = String::new();
        buf.extend(iter);
        buf
    }
}

impl<T, const N: usize> TryFrom<$UniqRc<[T]>> for $UniqRc<[T; N]> {
    type Error = <$Rc<[T; N]> as TryFrom<$Rc<[T]>>>::Error;

    fn try_from(this: $UniqRc<[T]>) -> Result<Self, Self::Error> {
        match this.rc.try_into() {
            Ok(rc) => unsafe {
                Ok(Self::new_unchecked(rc))
            },
            Err(e) => Err(e),
        }
    }
}

#[allow(clippy::from_over_into)]
impl<T: ?Sized> Into<$Rc<T>> for $UniqRc<T> {
    fn into(self) -> $Rc<T> {
        Self::into_rc(self)
    }
}

impl From<$UniqRc<str>> for $Rc<[u8]> {
    fn from(val: $UniqRc<str>) -> Self {
        val.rc.into()
    }
}

impl<T: ?Sized + Hasher> Hasher for $UniqRc<T> {
    fn finish(&self) -> u64 {
        (**self).finish()
    }
    fn write(&mut self, bytes: &[u8]) {
        (**self).write(bytes)
    }
    fn write_u8(&mut self, i: u8) {
        (**self).write_u8(i)
    }
    fn write_u16(&mut self, i: u16) {
        (**self).write_u16(i)
    }
    fn write_u32(&mut self, i: u32) {
        (**self).write_u32(i)
    }
    fn write_u64(&mut self, i: u64) {
        (**self).write_u64(i)
    }
    fn write_u128(&mut self, i: u128) {
        (**self).write_u128(i)
    }
    fn write_usize(&mut self, i: usize) {
        (**self).write_usize(i)
    }
    fn write_i8(&mut self, i: i8) {
        (**self).write_i8(i)
    }
    fn write_i16(&mut self, i: i16) {
        (**self).write_i16(i)
    }
    fn write_i32(&mut self, i: i32) {
        (**self).write_i32(i)
    }
    fn write_i64(&mut self, i: i64) {
        (**self).write_i64(i)
    }
    fn write_i128(&mut self, i: i128) {
        (**self).write_i128(i)
    }
    fn write_isize(&mut self, i: isize) {
        (**self).write_isize(i)
    }
}

impl<I: Iterator + ?Sized> Iterator for $UniqRc<I> {
    type Item = I::Item;

    fn next(&mut self) -> Option<I::Item> {
        (**self).next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (**self).size_hint()
    }

    fn nth(&mut self, n: usize) -> Option<I::Item> {
        (**self).nth(n)
    }

    fn last(self) -> Option<I::Item> {
        self.fold(None, |_, ele| Some(ele))
    }
}

impl<I: DoubleEndedIterator + ?Sized> DoubleEndedIterator for $UniqRc<I> {
    fn next_back(&mut self) -> Option<I::Item> {
        (**self).next_back()
    }

    fn nth_back(&mut self, n: usize) -> Option<I::Item> {
        (**self).nth_back(n)
    }
}

impl<I: ExactSizeIterator + ?Sized> ExactSizeIterator for $UniqRc<I> {
    fn len(&self) -> usize {
        (**self).len()
    }
}

impl<I: FusedIterator + ?Sized> FusedIterator for $UniqRc<I> {}

#[allow(clippy::non_send_fields_in_send_ty)]
unsafe impl<T: ?Sized> Send for $UniqRc<T>
where Box<T>: Send,
{
}

impl<F: ?Sized + Future + Unpin> Future for $UniqRc<F> {
    type Output = F::Output;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        F::poll(Pin::new(&mut *self), cx)
    }
}

#[cfg(feature = "std")]
impl<R: io::Read + ?Sized> io::Read for $UniqRc<R> {
    #[inline]
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        (**self).read(buf)
    }

    #[inline]
    fn read_vectored(&mut self, bufs: &mut [io::IoSliceMut<'_>]) -> io::Result<usize> {
        (**self).read_vectored(bufs)
    }

    #[inline]
    fn read_to_end(&mut self, buf: &mut alloc::vec::Vec<u8>) -> io::Result<usize> {
        (**self).read_to_end(buf)
    }

    #[inline]
    fn read_to_string(&mut self, buf: &mut String) -> io::Result<usize> {
        (**self).read_to_string(buf)
    }

    #[inline]
    fn read_exact(&mut self, buf: &mut [u8]) -> io::Result<()> {
        (**self).read_exact(buf)
    }
}

#[cfg(feature = "std")]
impl<S: io::Seek + ?Sized> io::Seek for $UniqRc<S> {
    #[inline]
    fn seek(&mut self, pos: io::SeekFrom) -> io::Result<u64> {
        (**self).seek(pos)
    }

    #[inline]
    fn stream_position(&mut self) -> io::Result<u64> {
        (**self).stream_position()
    }
}

#[cfg(feature = "std")]
impl<B: io::BufRead + ?Sized> io::BufRead for $UniqRc<B> {
    #[inline]
    fn fill_buf(&mut self) -> io::Result<&[u8]> {
        (**self).fill_buf()
    }

    #[inline]
    fn consume(&mut self, amt: usize) {
        (**self).consume(amt)
    }

    #[inline]
    fn read_until(&mut self, byte: u8, buf: &mut alloc::vec::Vec<u8>) -> io::Result<usize> {
        (**self).read_until(byte, buf)
    }

    #[inline]
    fn read_line(&mut self, buf: &mut String) -> io::Result<usize> {
        (**self).read_line(buf)
    }
}

#[cfg(feature = "std")]
impl<W: io::Write + ?Sized> io::Write for $UniqRc<W> {
    #[inline]
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        (**self).write(buf)
    }

    #[inline]
    fn write_vectored(&mut self, bufs: &[io::IoSlice<'_>]) -> io::Result<usize> {
        (**self).write_vectored(bufs)
    }

    #[inline]
    fn flush(&mut self) -> io::Result<()> {
        (**self).flush()
    }

    #[inline]
    fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
        (**self).write_all(buf)
    }

    #[inline]
    fn write_fmt(&mut self, fmt: fmt::Arguments<'_>) -> io::Result<()> {
        (**self).write_fmt(fmt)
    }
}

unsafe impl<T: ?Sized> Sync for $UniqRc<T>
where Box<T>: Sync,
{
}

impl<T: ?Sized> $UniqRc<T>
where $Rc<T>: MakeMut<T = T>,
{
    pub fn new(rc: $Rc<T>) -> Self {
        Self { rc: MakeMut::to_unique(rc) }
    }

    /// # Safety
    #[doc = concat!("- Compliant with the safety of [`", stringify!($Rc), "::from_raw`]")]
    pub unsafe fn from_raw(raw: *mut T) -> Self {
        unsafe {
            Self::new($Rc::from_raw(raw))
        }
    }
}

impl<T: ?Sized> $UniqRc<T> {
    /// # Errors
    /// - `rc` is shared, `strong_count != 1`
    pub fn try_new(mut rc: $Rc<T>) -> Result<Self, $Rc<T>> {
        if $Rc::get_mut(&mut rc).is_none() {
            return Err(rc);
        }

        unsafe {
            Ok(Self::new_unchecked(rc))
        }
    }

    /// # Safety
    /// - `strong_count == 1`
    /// - No `Weak` exists
    pub unsafe fn new_unchecked(rc: $Rc<T>) -> Self {
        debug_assert_eq!($Rc::strong_count(&rc), 1);
        Self { rc }
    }

    pub fn into_rc(this: Self) -> $Rc<T> {
        this.rc
    }

    /// # Safety
    /// - It is not allowed to change the strong and weak reference count
    pub unsafe fn get_rc_unchecked(this: &Self) -> &$Rc<T> {
        &this.rc
    }

    #[allow(clippy::ptr_cast_constness)]
    pub fn into_raw(this: Self) -> *mut T {
        $Rc::into_raw(this.rc) as *mut T
    }

    /// # Safety
    /// - Compliant with the safety of [`from_raw`](#method.from_raw)
    /// - Compliant with the safety of [`new_unchecked`](#method.new_unchecked)
    pub unsafe fn from_raw_unchecked(raw: *mut T) -> Self {
        unsafe {
            Self::new_unchecked($Rc::from_raw(raw))
        }
    }

    pub fn leak(this: Self) -> &'static mut T {
        let ptr = Self::into_raw(this);
        unsafe { &mut *ptr }
    }

    pub fn into_pin(this: Self) -> Pin<Self> {
        unsafe { Pin::new_unchecked(this) }
    }
}

impl<T> $UniqRc<T> {
    pub fn new_value(value: T) -> Self {
        Self { rc: $Rc::new(value) }
    }

    pub fn into_inner(this: Self) -> T {
        $Rc::try_unwrap(this.rc).ok().expect(concat!(
            "implement bug, inner ",
            stringify!($Rc),
            " strong_count != 1",
        ))
    }

    pub fn pin(data: T) -> Pin<Self> {
        unsafe { Pin::new_unchecked(Self::new_value(data)) }
    }
}

impl_downcast!($UniqRc:);
impl_downcast!($UniqRc: + Send);
impl_downcast!($UniqRc: + Send + Sync);
}}

impl_rc!(Rc,    UniqRc);
impl_rc!(Arc,   UniqArc);
