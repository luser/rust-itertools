//! Some iterator that produces tuples

use std::iter::Fuse;

/// An iterator over a incomplete tuple.
///
/// See [`.next_tuple()`](../trait.Itertools.html#method.tuples) and
/// [`Tuples::into_buffer()`](struct.Tuples.html#method.into_buffer).
pub struct TupleBuffer<T>
    where T: TupleCollect
{
    cur: usize,
    buf: T::Buffer,
}

impl<T> TupleBuffer<T>
    where T: TupleCollect
{
    fn new(buf: T::Buffer) -> Self {
        TupleBuffer {
            cur: 0,
            buf: buf,
        }
    }
}

impl<T> Iterator for TupleBuffer<T>
    where T: TupleCollect
{
    type Item = T::Item;

    fn next(&mut self) -> Option<Self::Item> {
        let s = self.buf.as_mut();
        if let Some(ref mut item) = s.get_mut(self.cur) {
            self.cur += 1;
            item.take()
        } else {
            None
        }
    }
}

/// An iterator that groups the items in tuples of a specific size.
///
/// See [`.tuples()`](../trait.Itertools.html#method.tuples) for more information.
pub struct Tuples<I, T>
    where I: Iterator<Item = T::Item>,
          T: TupleCollect
{
    iter: Fuse<I>,
    buf: T::Buffer,
}

/// Create a new tuples iterator.
pub fn tuples<I, T>(iter: I) -> Tuples<I, T>
    where I: Iterator<Item = T::Item>,
          T: TupleCollect
{
    Tuples {
        iter: iter.fuse(),
        buf: Default::default(),
    }
}

impl<I, T> Iterator for Tuples<I, T>
    where I: Iterator<Item = T::Item>,
          T: TupleCollect
{
    type Item = T;

    fn next(&mut self) -> Option<T> {
        T::collect_from_iter(&mut self.iter, &mut self.buf)
    }
}

impl<I, T> Tuples<I, T>
    where I: Iterator<Item = T::Item>,
          T: TupleCollect
{
    /// Return a buffer with the produced items that was not enough to be grouped in a tuple.
    ///
    /// ```
    /// use itertools::Itertools;
    ///
    /// let mut iter = (0..5).tuples();
    /// assert_eq!(Some((0, 1, 2)), iter.next());
    /// assert_eq!(None, iter.next());
    /// itertools::assert_equal(vec![3, 4], iter.into_buffer());
    /// ```
    pub fn into_buffer(self) -> TupleBuffer<T> {
        TupleBuffer::new(self.buf)
    }
}


/// An iterator over all contiguous windows that produces tuples of a specific size.
///
/// See [`.tuple_windows()`](../trait.Itertools.html#method.tuple_windows) for more
/// information.
pub struct TupleWindows<I, T>
    where I: Iterator<Item = T::Item>,
          T: TupleCollect
{
    iter: I,
    last: Option<T>,
}

/// Create a new tuple windows iterator.
pub fn tuple_windows<I, T>(mut iter: I) -> TupleWindows<I, T>
    where I: Iterator<Item = T::Item>,
          T: TupleCollect,
          T::Item: Clone
{
    use std::iter::once;

    let mut last = None;
    if T::num_items() != 1 {
        if let Some(item) = iter.next() {
            let iter = once(item.clone()).chain(once(item)).chain(&mut iter);
            last = T::collect_from_iter_no_buf(iter);
        }
    }

    TupleWindows {
        last: last,
        iter: iter,
    }
}

impl<I, T> Iterator for TupleWindows<I, T>
    where I: Iterator<Item = T::Item>,
          T: TupleCollect + Clone,
          T::Item: Clone
{
    type Item = T;

    fn next(&mut self) -> Option<T> {
        if T::num_items() == 1 {
            return T::collect_from_iter_no_buf(&mut self.iter)
        }
        if let Some(ref mut last) = self.last {
            if let Some(new) = self.iter.next() {
                last.left_shift_push(new);
                return Some(last.clone());
            }
        }
        None
    }
}

pub trait TupleCollect: Sized {
    type Item;
    type Buffer: Default + AsMut<[Option<Self::Item>]>;

    fn collect_from_iter<I>(iter: I, buf: &mut Self::Buffer) -> Option<Self>
        where I: IntoIterator<Item = Self::Item>;

    fn collect_from_iter_no_buf<I>(iter: I) -> Option<Self> where I: IntoIterator<Item = Self::Item>;

    fn try_collect_from_iter<I>(iter: I) -> Result<Self, TupleBuffer<Self>>
        where I: IntoIterator<Item = Self::Item>
    {
        let mut buf = Default::default();
        if let Some(t) = Self::collect_from_iter(iter, &mut buf) {
            Ok(t)
        } else {
            Err(TupleBuffer::new(buf))
        }
    }

    fn num_items() -> usize;

    fn left_shift_push(&mut self, item: Self::Item);
}

macro_rules! impl_tuple_collect {
    () => ();
    ($N:expr; $A:ident ; $($X:ident),* ; $($Y:ident),* ; $($Y_rev:ident),*) => (
        impl<$A> TupleCollect for ($($X),*,) {
            type Item = $A;
            type Buffer = [Option<$A>; $N - 1];

            #[allow(unused_assignments)]
            fn collect_from_iter<I>(iter: I, buf: &mut Self::Buffer) -> Option<Self>
                where I: IntoIterator<Item = $A>
            {
                let mut iter = iter.into_iter();
                $(
                    let mut $Y = None;
                )*

                loop {
                    $(
                        $Y = iter.next();
                        if $Y.is_none() {
                            break
                        }
                    )*
                    return Some(($($Y.unwrap()),*,))
                }

                let mut i = 0;
                let mut s = buf.as_mut();
                $(
                    if i < s.len() {
                        s[i] = $Y;
                        i += 1;
                    }
                )*
                return None;
            }

            #[allow(unused_assignments)]
            fn collect_from_iter_no_buf<I>(iter: I) -> Option<Self>
                where I: IntoIterator<Item = $A>
            {
                let mut iter = iter.into_iter();
                loop {
                    $(
                        let $Y = if let Some($Y) = iter.next() {
                            $Y
                        } else {
                            break;
                        };
                    )*
                    return Some(($($Y),*,))
                }

                return None;
            }

            fn num_items() -> usize {
                $N
            }

            fn left_shift_push(&mut self, item: $A) {
                use std::mem::replace;

                let &mut ($(ref mut $Y),*,) = self;
                let tmp = item;
                $(
                    let tmp = replace($Y_rev, tmp);
                )*
                drop(tmp);
            }
        }
    )
}

impl_tuple_collect!(1; A; A; a; a);
impl_tuple_collect!(2; A; A, A; a, b; b, a);
impl_tuple_collect!(3; A; A, A, A; a, b, c; c, b, a);
impl_tuple_collect!(4; A; A, A, A, A; a, b, c, d; d, c, b, a);
