#[derive(Debug, Clone, Copy)]
pub enum O<T> {
    Some(T),
    None,
}

impl<T> O<T> {
    /// Constructs an `O::Some(val)` variant.
    pub fn some(val: T) -> Self {
        O::Some(val)
    }

    /// Constructs an `O::None` variant.
    pub fn none() -> Self {
        O::None
    }

    /// Unwraps the value or panics with a byte-string message.
    pub fn unwrap_or_panic(self, _msg: &'static str) -> T {
        match self {
            O::Some(val) => val,
            O::None => panic!("could not unwrap 'O'"),
        }
    }

    /// Expects the value to be `Some`, panics with the given byte-string if not.
    pub fn expect(self, _msg: &'static str) -> T {
        match self {
            O::Some(val) => val,
            O::None => panic!("expected 'O::Some', got 'O::None'"),
        }
    }

    /// Returns true if the value is `Some`.
    pub fn is_some(&self) -> bool {
        matches!(self, O::Some(_))
    }

    /// Returns true if the value is `None`.
    pub fn is_none(&self) -> bool {
        matches!(self, O::None)
    }

    /// Maps `O<T>` to `O<U>` by applying `f` to the contained value.
    pub fn map<U, F: FnOnce(T) -> U>(self, f: F) -> O<U> {
        match self {
            O::Some(val) => O::Some(f(val)),
            O::None => O::None,
        }
    }

    /// Converts from `&O<T>` to `O<&T>`.
    pub fn as_ref(&self) -> O<&T> {
        match self {
            O::Some(val) => O::Some(val),
            O::None => O::None,
        }
    }

    /// Converts from `&mut O<T>` to `O<&mut T>`.
    pub fn as_mut(&mut self) -> O<&mut T> {
        match self {
            O::Some(val) => O::Some(val),
            O::None => O::None,
        }
    }
}
