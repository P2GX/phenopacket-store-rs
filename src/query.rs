pub trait Query<T>
where
    T: ?Sized,
{
    fn test(&self, item: &T) -> bool;

    fn and<O>(self, other: O) -> AndQuery<Self, O>
    where
        Self: Sized,
    {
        AndQuery(self, other)
    }
}

impl<T, V> Query<V> for &T
where
    T: Query<V>,
    V: ?Sized,
{
    fn test(&self, item: &V) -> bool {
        (*self).test(item)
    }
}

impl<T, V> Query<V> for Box<T>
where
    T: Query<V>,
    V: ?Sized,
{
    fn test(&self, item: &V) -> bool {
        (**self).test(item)
    }
}

/* Boolean algebra */

pub struct AndQuery<P, R>(pub P, pub R);

impl<P, R> AndQuery<P, R> {
    pub fn new(p: P, r: R) -> Self {
        Self(p, r)
    }
}

impl<P, R, T> Query<T> for AndQuery<P, R>
where
    P: Query<T>,
    R: Query<T>,
    T: ?Sized,
{
    fn test(&self, item: &T) -> bool {
        self.0.test(item) && self.1.test(item)
    }
}

pub struct OrQuery<P, R>(P, R);

impl<P, R> OrQuery<P, R> {
    pub fn new(p: P, r: R) -> Self {
        Self(p, r)
    }
}

impl<P, R, T> Query<T> for OrQuery<P, R>
where
    P: Query<T>,
    R: Query<T>,
    T: ?Sized,
{
    fn test(&self, item: &T) -> bool {
        self.0.test(item) || self.1.test(item)
    }
}

pub struct NotQuery<P>(P);

impl<P> NotQuery<P> {
    pub fn new(p: P) -> Self {
        Self(p)
    }
}

impl<P, T> Query<T> for NotQuery<P>
where
    P: Query<T>,
    T: ?Sized,
{
    fn test(&self, item: &T) -> bool {
        !self.0.test(item)
    }
}

#[macro_export]
macro_rules! impl_bool_algebra {
    // Match the generic queries
    ( $ty:ident < $( $gen:tt ),+ > ) => {
        impl<$($gen)+, P> std::ops::BitAnd<P> for $ty<$($gen)+> {
            type Output = $crate::query::AndQuery<$ty<$($gen)+>, P>;

            fn bitand(self, rhs: P) -> Self::Output {
                $crate::query::AndQuery::new(self, rhs)
            }
        }

        impl<'_ref_, $($gen)+, P> std::ops::BitAnd<P> for &'_ref_ $ty<$($gen)+> {
            type Output = $crate::query::AndQuery<&'_ref_ $ty<$($gen)+>, P>;

            fn bitand(self, rhs: P) -> Self::Output {
                $crate::query::AndQuery::new(self, rhs)
            }
        }

        impl<$($gen)+, P> std::ops::BitOr<P> for $ty<$($gen)+> {
            type Output = $crate::query::OrQuery<$ty<$($gen)+>, P>;

            fn bitor(self, rhs: P) -> Self::Output {
                $crate::query::OrQuery::new(self, rhs)
            }
        }

        impl<'_ref_, $($gen)+, P> std::ops::BitOr<P> for &'_ref_ $ty<$($gen)+> {
            type Output = $crate::query::OrQuery<&'_ref_ $ty<$($gen)+>, P>;

            fn bitor(self, rhs: P) -> Self::Output {
                $crate::query::OrQuery::new(self, rhs)
            }
        }

        impl<$($gen)+,> std::ops::Not for $ty<$($gen)+> {
            type Output = $crate::query::NotQuery<$ty<$($gen)+>>;

            fn not(self) -> Self::Output {
                $crate::query::NotQuery::new(self)
            }
        }

        impl<'_ref_, $($gen)+,> std::ops::Not for &'_ref_ $ty<$($gen)+> {
            type Output = $crate::query::NotQuery<&'_ref_ $ty<$($gen)+>>;

            fn not(self) -> Self::Output {
                $crate::query::NotQuery::new(self)
            }
        }
    };

    // Match the non-generic types
    ( $ty:ident ) => {
        impl<P> std::ops::BitAnd<P> for $ty {
            type Output = $crate::query::AndQuery<$ty, P>;

            fn bitand(self, rhs: P) -> Self::Output {
                $crate::query::AndQuery::new(self, rhs)
            }
        }

        impl<'a, P> std::ops::BitAnd<P> for &'a $ty {
            type Output = $crate::query::AndQuery<&'a $ty, P>;

            fn bitand(self, rhs: P) -> Self::Output {
                $crate::query::AndQuery::new(self, rhs)
            }
        }

        impl<P> std::ops::BitOr<P> for $ty {
            type Output = $crate::query::OrQuery<$ty, P>;

            fn bitor(self, rhs: P) -> Self::Output {
                $crate::query::OrQuery::new(self, rhs)
            }
        }

        impl<'a, P> std::ops::BitOr<P> for &'a $ty {
            type Output = $crate::query::OrQuery<&'a $ty, P>;

            fn bitor(self, rhs: P) -> Self::Output {
                $crate::query::OrQuery::new(self, rhs)
            }
        }

        impl std::ops::Not for $ty {
            type Output = $crate::query::NotQuery<$ty>;

            fn not(self) -> Self::Output {
                $crate::query::NotQuery::new(self)
            }
        }

        impl<'a> std::ops::Not for &'a $ty {
            type Output = $crate::query::NotQuery<&'a $ty>;

            fn not(self) -> Self::Output {
                $crate::query::NotQuery::new(self)
            }
        }
    };
}
pub use impl_bool_algebra;

#[cfg(test)]
mod test_predicate {
    use super::Query;

    #[derive(Clone)]
    pub struct Foo;
    impl<T> Query<T> for Foo {
        fn test(&self, _item: &T) -> bool {
            false
        }
    }
    crate::impl_bool_algebra!(Foo);

    #[test]
    fn test_bool_algebra() {
        let a = Foo;
        let b = Foo;
        let val = 123u8;

        let x = &a & &b;
        assert!(!x.test(&val));

        let y = &a | &b;
        assert!(!y.test(&val));

        let z = !&a;
        assert!(z.test(&val));
    }
}
