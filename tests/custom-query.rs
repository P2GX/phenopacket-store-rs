use ppktstore::{impl_bool_algebra, query::Query};

#[derive(Debug, Clone)]
struct IsLowercase;

impl Query<str> for IsLowercase {
    fn test(&self, item: &str) -> bool {
        item.chars().all(|c| c.is_lowercase())
    }
}
impl_bool_algebra!(IsLowercase);

#[derive(Debug, Clone)]
struct StartsWith<'a>(&'a str);
impl<'a> Query<str> for StartsWith<'a> {
    fn test(&self, item: &str) -> bool {
        item.starts_with(self.0)
    }
}
impl_bool_algebra!(StartsWith<'a>);

#[test]
fn test_boolean_queries() {
    let is_lowercase = IsLowercase;
    let starts_with_abc = StartsWith("abc");

    let lowercase_and_startswith_abc = &is_lowercase & &starts_with_abc;
    assert!(lowercase_and_startswith_abc.test("abcdef"));
    assert!(!lowercase_and_startswith_abc.test("abcDEF"));
    assert!(!lowercase_and_startswith_abc.test("ABC"));

    let lowercase_or_startswith_abc = &is_lowercase | &starts_with_abc;
    assert!(lowercase_or_startswith_abc.test("abcdef"));
    assert!(lowercase_or_startswith_abc.test("abcDEF"));
    assert!(!lowercase_or_startswith_abc.test("ABC"));

    let not_lowercase = !&is_lowercase;
    assert!(!not_lowercase.test("abcdef"));
    assert!(not_lowercase.test("abcDEF"));
    assert!(not_lowercase.test("ABC"));
}
