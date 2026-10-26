/// A query for testing if a value with type `T` meets a condition.
///
/// The users are encouraged to implement basic queries for simple tests,
/// and to combine basic queries into complex tests using the boolean algebra operators, including `&` (and), `|` (or), and `!` (not).
///
/// # Examples
///
/// We can implement a query to test if a `&str` consists of lower-case characters.
///
/// ```
/// use ppktstore::{impl_bool_algebra, query::Query};
///
/// #[derive(Debug, Clone)]
/// struct IsLowercase;
///
/// impl Query<str> for IsLowercase {  // ❶
///     fn test(&self, item: &str) -> bool {
///         item.chars().all(|c| c.is_lowercase())  // ➋
///     }
/// }
/// impl_bool_algebra!(IsLowercase); // ➌
/// ```
///
/// The `Query` trait must be implemented (❶) such that `true` is returned (➋) if the `item` matches the condition,
/// and `false` otherwise.
///
/// The [`impl_bool_algebra`] macro enables (➌) building complex queries from simple building blocks
/// using the boolean algebra operators.
pub trait Query<T>
where
    T: ?Sized,
{
    fn test(&self, item: &T) -> bool;
}

impl<Q, T> Query<T> for &Q
where
    Q: Query<T>,
    T: ?Sized,
{
    fn test(&self, item: &T) -> bool {
        (*self).test(item)
    }
}

impl<Q, T> Query<T> for Box<Q>
where
    Q: Query<T>,
    T: ?Sized,
{
    fn test(&self, item: &T) -> bool {
        (**self).test(item)
    }
}

/* Boolean algebra */
#[derive(Clone, Debug)]
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
impl_bool_algebra!(AndQuery<P, R>);

#[derive(Clone, Debug)]
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
impl_bool_algebra!(OrQuery<P, R>);

#[derive(Clone, Debug)]
pub struct NotQuery<Q>(Q);

impl<P> NotQuery<P> {
    pub fn new(p: P) -> Self {
        Self(p)
    }
}

impl<Q, T> Query<T> for NotQuery<Q>
where
    Q: Query<T>,
    T: ?Sized,
{
    fn test(&self, item: &T) -> bool {
        !self.0.test(item)
    }
}
impl_bool_algebra!(NotQuery<Q>);

/// A macro for implementing the boolean algebra operators for an arbitrary [`Query`].
#[macro_export]
macro_rules! impl_bool_algebra {

    // Queries with lifetimes and generic params
    ( $ty:ident < $($gen:tt),* > ) => {
        impl< $($gen),* , __P> std::ops::BitAnd<__P> for $ty< $($gen),* > {
            type Output = $crate::query::AndQuery<$ty< $($gen),* >, __P>;

            fn bitand(self, rhs: __P) -> Self::Output {
                $crate::query::AndQuery::new(self, rhs)
            }
        }

        impl<'__qref, $($gen),*, __P> std::ops::BitAnd<__P> for &'__qref $ty< $($gen),* > {
            type Output = $crate::query::AndQuery<&'__qref $ty< $($gen),* >, __P>;

            fn bitand(self, rhs: __P) -> Self::Output {
                $crate::query::AndQuery::new(self, rhs)
            }
        }

        impl< $($gen),*, __P> std::ops::BitOr<__P> for $ty< $($gen),* > {
            type Output = $crate::query::OrQuery<$ty< $($gen),* >, __P>;

            fn bitor(self, rhs: __P) -> Self::Output {
                $crate::query::OrQuery::new(self, rhs)
            }
        }

        impl<'__qref, $($gen),*, __P> std::ops::BitOr<__P> for &'__qref $ty< $($gen),* > {
            type Output = $crate::query::OrQuery<&'__qref $ty< $($gen),* >, __P>;

            fn bitor(self, rhs: __P) -> Self::Output {
                $crate::query::OrQuery::new(self, rhs)
            }
        }

        impl< $($gen),* > std::ops::Not for $ty< $($gen),* > {
            type Output = $crate::query::NotQuery<$ty< $($gen),* >>;

            fn not(self) -> Self::Output {
                $crate::query::NotQuery::new(self)
            }
        }

        impl<'__qref, $($gen),* > std::ops::Not for &'__qref $ty< $($gen),* > {
            type Output = $crate::query::NotQuery<&'__qref $ty< $($gen),*>>;

            fn not(self) -> Self::Output {
                $crate::query::NotQuery::new(self)
            }
        }
    };

    // Match the non-generic types
    ( $ty:ty ) => {
        impl<__P> std::ops::BitAnd<__P> for $ty {
            type Output = $crate::query::AndQuery<$ty, __P>;

            fn bitand(self, rhs: __P) -> Self::Output {
                $crate::query::AndQuery::new(self, rhs)
            }
        }

        impl<'__qref, __P> std::ops::BitAnd<__P> for &'__qref $ty {
            type Output = $crate::query::AndQuery<&'__qref $ty, __P>;

            fn bitand(self, rhs: __P) -> Self::Output {
                $crate::query::AndQuery::new(self, rhs)
            }
        }

        impl<__P> std::ops::BitOr<__P> for $ty {
            type Output = $crate::query::OrQuery<$ty, __P>;

            fn bitor(self, rhs: __P) -> Self::Output {
                $crate::query::OrQuery::new(self, rhs)
            }
        }

        impl<'__qref, __P> std::ops::BitOr<__P> for &'__qref $ty {
            type Output = $crate::query::OrQuery<&'__qref $ty, __P>;

            fn bitor(self, rhs: __P) -> Self::Output {
                $crate::query::OrQuery::new(self, rhs)
            }
        }

        impl std::ops::Not for $ty {
            type Output = $crate::query::NotQuery<$ty>;

            fn not(self) -> Self::Output {
                $crate::query::NotQuery::new(self)
            }
        }

        impl<'__qref> std::ops::Not for &'__qref $ty {
            type Output = $crate::query::NotQuery<&'__qref $ty>;

            fn not(self) -> Self::Output {
                $crate::query::NotQuery::new(self)
            }
        }
    };
}
pub use impl_bool_algebra;

#[cfg(test)]
mod test_query {
    use super::Query;

    struct Foo(bool);
    impl<T> Query<T> for Foo {
        fn test(&self, _item: &T) -> bool {
            self.0
        }
    }
    crate::impl_bool_algebra!(Foo);

    #[test]
    fn test_bool_algebra_and() {
        let a = Foo(true);
        let b = Foo(false);
        let val = 123u8;

        let x = &a & &a;
        assert!(x.test(&val));
        let x = &a & &b;
        assert!(!x.test(&val));
        let x = &b & &b;
        assert!(!x.test(&val));
    }

    #[test]
    fn test_bool_algebra_or() {
        let a = Foo(true);
        let b = Foo(false);
        let val = 123u8;

        let x = &a | &a;
        assert!(x.test(&val));
        let x = &a | &b;
        assert!(x.test(&val));
        let x = &b | &b;
        assert!(!x.test(&val));
    }

    #[test]
    fn test_bool_algebra_not() {
        let a = Foo(true);
        let val = 123u8;

        assert!(a.test(&val));
        assert!(!(!a).test(&val));
    }

    #[test]
    fn double_negation_query() {
        let a = Foo(true);
        let _x = !!a;
    }

    #[test]
    fn queries_can_be_chained() {
        let a = Foo(true);
        let b = Foo(false);
        let c = Foo(false);

        let _x = !a & b & c;
    }

    #[test]
    fn impl_bool_algebra_for_a_type_with_multiple_generic_params() {
        struct Bar<A, B, C>(A, B, C);
        impl_bool_algebra!(Bar<A, B, C>);
    }

    #[test]
    #[allow(dead_code)]
    fn impl_bool_algebra_for_a_type_with_multiple_lifetimes() {
        struct Bar<'a, 'b>(&'a u8, &'b u8);
        impl_bool_algebra!(Bar<'a, 'b>);
    }

    #[test]
    #[allow(dead_code)]
    fn impl_bool_algebra_for_a_type_with_lifetimes_and_generic_params() {
        struct Bar<'a, 'b, A, B, C>(&'a A, &'b B, C);
        impl_bool_algebra!(Bar<'a, 'b, A, B, C>);
    }

    #[test]
    #[allow(dead_code)]
    fn query_is_dyn_compatible() {
        fn bla<T>(_v: &dyn Query<T>) {}
    }
}
