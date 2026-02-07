//! Primitive wrapper contexts (String, Number, Array).

use githook_macros::{callable_impl, docs};

/// Typed context for `String` method dispatch (`length`, `upper`, `contains`, …).
#[derive(Debug, Clone)]
pub struct StringContext {
    value: String,
}

impl StringContext {
    /// Wraps a raw string.
    pub fn new(value: String) -> Self {
        Self { value }
    }

    /// Returns the inner value by reference.
    pub fn value(&self) -> &str {
        &self.value
    }
}

#[callable_impl]
impl StringContext {
    #[docs(
        name = "string.length",
        description = "Length of the string",
        example = "print \"hello\".length"
    )]
    #[property]
    pub fn length(&self) -> f64 {
        self.value.len() as f64
    }

    #[docs(
        name = "string.upper",
        description = "Converts string to uppercase",
        example = "\"hello\".upper // \"HELLO\""
    )]
    #[property]
    pub fn upper(&self) -> String {
        self.value.to_uppercase()
    }

    #[docs(
        name = "string.lower",
        description = "Converts string to lowercase",
        example = "\"HELLO\".lower // \"hello\""
    )]
    #[property]
    pub fn lower(&self) -> String {
        self.value.to_lowercase()
    }

    #[docs(
        name = "string.reverse",
        description = "Reverses the string",
        example = "\"hello\".reverse // \"olleh\""
    )]
    #[method]
    pub fn reverse(&self) -> String {
        self.value.chars().rev().collect()
    }

    #[docs(
        name = "string.len",
        description = "Returns the length of the string",
        example = "\"hello\".len // 5"
    )]
    #[method]
    pub fn len(&self) -> f64 {
        self.value.len() as f64
    }

    #[docs(
        name = "string.is_empty",
        description = "Checks if the string is empty",
        example = "\"\".is_empty // true"
    )]
    #[method]
    pub fn is_empty(&self) -> bool {
        self.value.is_empty()
    }

    #[docs(
        name = "string.to_lowercase",
        description = "Converts string to lowercase",
        example = "\"HELLO\".to_lowercase() // \"hello\""
    )]
    #[method]
    pub fn to_lowercase(&self) -> String {
        self.value.to_lowercase()
    }

    #[docs(
        name = "string.to_uppercase",
        description = "Converts string to uppercase",
        example = "\"hello\".to_uppercase() // \"HELLO\""
    )]
    #[method]
    pub fn to_uppercase(&self) -> String {
        self.value.to_uppercase()
    }

    #[docs(
        name = "string.trim",
        description = "Removes leading and trailing whitespace",
        example = "\"  hello  \".trim // \"hello\""
    )]
    #[method]
    pub fn trim(&self) -> String {
        self.value.trim().to_string()
    }

    #[docs(
        name = "string.replace",
        description = "Replaces occurrences of a substring with another",
        example = "\"hello world\".replace(\"world\", \"there\") // \"hello there\""
    )]
    #[method]
    pub fn replace(&self, from: &str, to: &str) -> String {
        self.value.replace(from, to)
    }

    #[docs(
        name = "string.contains",
        description = "Checks if the string contains a substring",
        example = "\"hello world\".contains(\"world\") // true"
    )]
    #[method]
    pub fn contains(&self, needle: &str) -> bool {
        self.value.contains(needle)
    }

    #[docs(
        name = "string.starts_with",
        description = "Checks if the string starts with a prefix",
        example = "\"hello world\".starts_with(\"hello\") // true"
    )]
    #[method]
    pub fn starts_with(&self, prefix: &str) -> bool {
        self.value.starts_with(prefix)
    }

    #[docs(
        name = "string.ends_with",
        description = "Checks if the string ends with a suffix",
        example = "\"hello world\".ends_with(\"world\") // true"
    )]
    #[method]
    pub fn ends_with(&self, suffix: &str) -> bool {
        self.value.ends_with(suffix)
    }

    #[docs(
        name = "string.matches",
        description = "Checks if the string matches a regex pattern",
        example = "\"hello123\".matches(\"^hello\\\\d+$\") // true"
    )]
    #[method]
    pub fn matches(&self, pattern: &str) -> bool {
        regex::Regex::new(pattern)
            .map(|re| re.is_match(&self.value))
            .unwrap_or(false)
    }

    #[docs(
        name = "string.split",
        description = "Splits the string by a delimiter",
        example = "\"a,b,c\".split(\",\") // [\"a\", \"b\", \"c\"]"
    )]
    #[method]
    pub fn split(&self, delimiter: &str) -> Vec<String> {
        self.value.split(delimiter).map(|s| s.to_string()).collect()
    }

    #[docs(
        name = "string.lines",
        description = "Splits the string into lines",
        example = "\"line1\\nline2\".lines // [\"line1\", \"line2\"]"
    )]
    #[method]
    pub fn lines(&self) -> Vec<String> {
        self.value.lines().map(|s| s.to_string()).collect()
    }

    #[docs(
        name = "string.slice",
        description = "Returns a substring from start to end index (exclusive). Negative indices count from the end.",
        example = "\"hello\".slice(1, 3) // \"el\""
    )]
    #[method]
    pub fn slice(&self, start: f64, end: f64) -> String {
        let len = self.value.chars().count() as i64;
        let s = start as i64;
        let e = end as i64;
        let s = if s < 0 { (len + s).max(0) } else { s.min(len) } as usize;
        let e = if e < 0 { (len + e).max(0) } else { e.min(len) } as usize;
        if s >= e {
            String::new()
        } else {
            self.value.chars().skip(s).take(e - s).collect()
        }
    }
}

/// Typed context for `Number` method dispatch (`abs`, `floor`, `sqrt`, …).
#[derive(Debug, Clone)]
pub struct NumberContext {
    value: f64,
}

impl NumberContext {
    /// Wraps a raw f64.
    pub fn new(value: f64) -> Self {
        Self { value }
    }

    /// Returns the inner value.
    pub fn value(&self) -> f64 {
        self.value
    }
}

#[callable_impl]
impl NumberContext {
    #[docs(
        name = "number.abs",
        description = "Absolute value of the number",
        example = "print (-5).abs"
    )]
    #[method]
    pub fn abs(&self) -> f64 {
        self.value.abs()
    }

    #[docs(
        name = "number.floor",
        description = "Floor of the number",
        example = "print 3.7.floor"
    )]
    #[method]
    pub fn floor(&self) -> f64 {
        self.value.floor()
    }

    #[docs(
        name = "number.ceil",
        description = "Ceiling of the number",
        example = "print 3.3.ceil"
    )]
    #[method]
    pub fn ceil(&self) -> f64 {
        self.value.ceil()
    }

    #[docs(
        name = "number.round",
        description = "Rounds the number to the nearest integer",
        example = "print 3.5.round"
    )]
    #[method]
    pub fn round(&self) -> f64 {
        self.value.round()
    }

    #[docs(
        name = "number.sqrt",
        description = "Square root of the number",
        example = "print 16.sqrt // 4"
    )]
    #[method]
    pub fn sqrt(&self) -> f64 {
        self.value.sqrt()
    }

    #[docs(
        name = "number.pow",
        description = "Raises the number to the power of exp",
        example = "print 2.pow(3) // 8"
    )]
    #[method]
    pub fn pow(&self, exp: f64) -> f64 {
        self.value.powf(exp)
    }

    #[docs(
        name = "number.sin",
        description = "Sine of the number (in radians)",
        example = "print (3.14159 / 2).sin() // ~1"
    )]
    #[method]
    pub fn sin(&self) -> f64 {
        self.value.sin()
    }

    #[docs(
        name = "number.cos",
        description = "Cosine of the number (in radians)",
        example = "print 0.0.cos() // 1"
    )]
    #[method]
    pub fn cos(&self) -> f64 {
        self.value.cos()
    }

    #[docs(
        name = "number.tan",
        description = "Tangent of the number (in radians)",
        example = "print 0.0.tan() // 0"
    )]
    #[method]
    pub fn tan(&self) -> f64 {
        self.value.tan()
    }

    #[docs(
        name = "number.percent",
        description = "Converts a decimal number to percentage",
        example = "print 0.85.percent() // 85.0"
    )]
    #[method]
    pub fn percent(&self) -> f64 {
        self.value * 100.0
    }
}

/// Typed context for `Array` method dispatch (`length`, `first`, `filter`, …).
#[derive(Debug, Clone)]
pub struct ArrayContext {
    items: Vec<crate::value::Value>,
}

impl ArrayContext {
    /// Wraps a vector of values.
    pub fn new(items: Vec<crate::value::Value>) -> Self {
        Self { items }
    }

    /// Returns the inner items by reference.
    pub fn items(&self) -> &[crate::value::Value] {
        &self.items
    }
}

#[callable_impl]
impl ArrayContext {
    #[docs(
        name = "array.length",
        description = "Length of the array",
        example = "print my_array.length"
    )]
    #[property]
    pub fn length(&self) -> f64 {
        self.items.len() as f64
    }

    #[docs(
        name = "array.first",
        description = "Returns the first element of the array",
        example = "print my_array.first"
    )]
    #[method]
    pub fn first(&self) -> String {
        self.items
            .first()
            .map(|v| format!("{:?}", v))
            .unwrap_or_else(|| "null".to_string())
    }

    #[docs(
        name = "array.last",
        description = "Returns the last element of the array",
        example = "print my_array.last"
    )]
    #[method]
    pub fn last(&self) -> String {
        self.items
            .last()
            .map(|v| format!("{:?}", v))
            .unwrap_or_else(|| "null".to_string())
    }

    #[docs(
        name = "array.is_empty",
        description = "Checks if the array is empty",
        example = "if my_array.is_empty { print \"Array is empty\" }"
    )]
    #[method]
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    #[docs(
        name = "array.sum",
        description = "Calculates the sum of numeric elements in the array",
        example = "print my_array.sum"
    )]
    #[method]
    pub fn sum(&self) -> f64 {
        use crate::value::Value;
        self.items
            .iter()
            .filter_map(|v| match v {
                Value::Number(n) => Some(*n),
                _ => None,
            })
            .sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::value::Value;

    // ── StringContext ──────────────────────────────────────────

    #[test]
    fn string_length() {
        let ctx = StringContext::new("hello".into());
        assert_eq!(ctx.length(), 5.0);
    }

    #[test]
    fn string_length_empty() {
        let ctx = StringContext::new(String::new());
        assert_eq!(ctx.length(), 0.0);
    }

    #[test]
    fn string_upper() {
        let ctx = StringContext::new("hello".into());
        assert_eq!(ctx.upper(), "HELLO");
    }

    #[test]
    fn string_lower() {
        let ctx = StringContext::new("HELLO".into());
        assert_eq!(ctx.lower(), "hello");
    }

    #[test]
    fn string_reverse() {
        let ctx = StringContext::new("hello".into());
        assert_eq!(ctx.reverse(), "olleh");
    }

    #[test]
    fn string_len() {
        let ctx = StringContext::new("abc".into());
        assert_eq!(ctx.len(), 3.0);
    }

    #[test]
    fn string_is_empty_true() {
        let ctx = StringContext::new(String::new());
        assert!(ctx.is_empty());
    }

    #[test]
    fn string_is_empty_false() {
        let ctx = StringContext::new("x".into());
        assert!(!ctx.is_empty());
    }

    #[test]
    fn string_to_lowercase() {
        let ctx = StringContext::new("HeLLo".into());
        assert_eq!(ctx.to_lowercase(), "hello");
    }

    #[test]
    fn string_to_uppercase() {
        let ctx = StringContext::new("HeLLo".into());
        assert_eq!(ctx.to_uppercase(), "HELLO");
    }

    #[test]
    fn string_trim() {
        let ctx = StringContext::new("  hello  ".into());
        assert_eq!(ctx.trim(), "hello");
    }

    #[test]
    fn string_replace() {
        let ctx = StringContext::new("hello world".into());
        assert_eq!(ctx.replace("world", "there"), "hello there");
    }

    #[test]
    fn string_contains_true() {
        let ctx = StringContext::new("hello world".into());
        assert!(ctx.contains("world"));
    }

    #[test]
    fn string_contains_false() {
        let ctx = StringContext::new("hello world".into());
        assert!(!ctx.contains("xyz"));
    }

    #[test]
    fn string_starts_with() {
        let ctx = StringContext::new("hello world".into());
        assert!(ctx.starts_with("hello"));
        assert!(!ctx.starts_with("world"));
    }

    #[test]
    fn string_ends_with() {
        let ctx = StringContext::new("hello world".into());
        assert!(ctx.ends_with("world"));
        assert!(!ctx.ends_with("hello"));
    }

    #[test]
    fn string_matches_valid_pattern() {
        let ctx = StringContext::new("hello123".into());
        assert!(ctx.matches(r"^hello\d+$"));
    }

    #[test]
    fn string_matches_invalid_pattern() {
        let ctx = StringContext::new("hello".into());
        // Invalid regex → returns false, not a panic
        assert!(!ctx.matches("[invalid"));
    }

    #[test]
    fn string_split() {
        let ctx = StringContext::new("a,b,c".into());
        assert_eq!(ctx.split(","), vec!["a", "b", "c"]);
    }

    #[test]
    fn string_lines() {
        let ctx = StringContext::new("line1\nline2\nline3".into());
        assert_eq!(ctx.lines(), vec!["line1", "line2", "line3"]);
    }

    #[test]
    fn string_value_accessor() {
        let ctx = StringContext::new("test".into());
        assert_eq!(ctx.value(), "test");
    }

    // ── NumberContext ──────────────────────────────────────────

    #[test]
    fn number_abs_positive() {
        let ctx = NumberContext::new(5.0);
        assert_eq!(ctx.abs(), 5.0);
    }

    #[test]
    fn number_abs_negative() {
        let ctx = NumberContext::new(-5.0);
        assert_eq!(ctx.abs(), 5.0);
    }

    #[test]
    fn number_floor() {
        let ctx = NumberContext::new(3.7);
        assert_eq!(ctx.floor(), 3.0);
    }

    #[test]
    fn number_ceil() {
        let ctx = NumberContext::new(3.3);
        assert_eq!(ctx.ceil(), 4.0);
    }

    #[test]
    fn number_round() {
        let ctx = NumberContext::new(3.5);
        assert_eq!(ctx.round(), 4.0);
    }

    #[test]
    fn number_sqrt() {
        let ctx = NumberContext::new(16.0);
        assert_eq!(ctx.sqrt(), 4.0);
    }

    #[test]
    fn number_pow() {
        let ctx = NumberContext::new(2.0);
        assert_eq!(ctx.pow(3.0), 8.0);
    }

    #[test]
    fn number_sin() {
        let ctx = NumberContext::new(0.0);
        assert!((ctx.sin() - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn number_cos() {
        let ctx = NumberContext::new(0.0);
        assert!((ctx.cos() - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn number_tan() {
        let ctx = NumberContext::new(0.0);
        assert!((ctx.tan() - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn number_percent() {
        let ctx = NumberContext::new(0.85);
        assert!((ctx.percent() - 85.0).abs() < f64::EPSILON);
    }

    #[test]
    fn number_value_accessor() {
        let ctx = NumberContext::new(42.0);
        assert_eq!(ctx.value(), 42.0);
    }

    // ── ArrayContext ──────────────────────────────────────────

    #[test]
    fn array_length() {
        let ctx = ArrayContext::new(vec![Value::Number(1.0), Value::Number(2.0)]);
        assert_eq!(ctx.length(), 2.0);
    }

    #[test]
    fn array_length_empty() {
        let ctx = ArrayContext::new(vec![]);
        assert_eq!(ctx.length(), 0.0);
    }

    #[test]
    fn array_first() {
        let ctx = ArrayContext::new(vec![Value::String("a".into()), Value::String("b".into())]);
        let first = ctx.first();
        assert!(first.contains('a'));
    }

    #[test]
    fn array_first_empty() {
        let ctx = ArrayContext::new(vec![]);
        assert_eq!(ctx.first(), "null");
    }

    #[test]
    fn array_last() {
        let ctx = ArrayContext::new(vec![Value::String("a".into()), Value::String("b".into())]);
        let last = ctx.last();
        assert!(last.contains('b'));
    }

    #[test]
    fn array_last_empty() {
        let ctx = ArrayContext::new(vec![]);
        assert_eq!(ctx.last(), "null");
    }

    #[test]
    fn array_is_empty_true() {
        let ctx = ArrayContext::new(vec![]);
        assert!(ctx.is_empty());
    }

    #[test]
    fn array_is_empty_false() {
        let ctx = ArrayContext::new(vec![Value::Null]);
        assert!(!ctx.is_empty());
    }

    #[test]
    fn array_sum_numbers() {
        let ctx = ArrayContext::new(vec![
            Value::Number(1.0),
            Value::Number(2.0),
            Value::Number(3.0),
        ]);
        assert_eq!(ctx.sum(), 6.0);
    }

    #[test]
    fn array_sum_mixed_types() {
        let ctx = ArrayContext::new(vec![
            Value::Number(10.0),
            Value::String("skip".into()),
            Value::Number(5.0),
        ]);
        // Non-numeric values are ignored
        assert_eq!(ctx.sum(), 15.0);
    }

    #[test]
    fn array_sum_empty() {
        let ctx = ArrayContext::new(vec![]);
        assert_eq!(ctx.sum(), 0.0);
    }

    #[test]
    fn array_items_accessor() {
        let items = vec![Value::Bool(true), Value::Null];
        let ctx = ArrayContext::new(items);
        assert_eq!(ctx.items().len(), 2);
    }

    // ── StringContext::slice ───────────────────────────────────

    #[test]
    fn string_slice_basic() {
        let ctx = StringContext::new("hello".into());
        assert_eq!(ctx.slice(1.0, 3.0), "el");
    }

    #[test]
    fn string_slice_full() {
        let ctx = StringContext::new("hello".into());
        assert_eq!(ctx.slice(0.0, 5.0), "hello");
    }

    #[test]
    fn string_slice_empty_range() {
        let ctx = StringContext::new("hello".into());
        assert_eq!(ctx.slice(3.0, 3.0), "");
    }

    #[test]
    fn string_slice_reversed_range() {
        let ctx = StringContext::new("hello".into());
        assert_eq!(ctx.slice(4.0, 2.0), "");
    }

    #[test]
    fn string_slice_negative_end() {
        let ctx = StringContext::new("hello".into());
        assert_eq!(ctx.slice(1.0, -1.0), "ell");
    }

    #[test]
    fn string_slice_negative_start() {
        let ctx = StringContext::new("hello".into());
        assert_eq!(ctx.slice(-3.0, 5.0), "llo");
    }

    #[test]
    fn string_slice_both_negative() {
        let ctx = StringContext::new("hello".into());
        assert_eq!(ctx.slice(-4.0, -1.0), "ell");
    }

    #[test]
    fn string_slice_out_of_bounds() {
        let ctx = StringContext::new("hi".into());
        assert_eq!(ctx.slice(0.0, 100.0), "hi");
    }

    #[test]
    fn string_slice_empty_string() {
        let ctx = StringContext::new("".into());
        assert_eq!(ctx.slice(0.0, 1.0), "");
    }

    #[test]
    fn string_slice_unicode() {
        let ctx = StringContext::new("héllo".into());
        assert_eq!(ctx.slice(0.0, 2.0), "hé");
    }
}
