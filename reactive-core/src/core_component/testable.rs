pub trait Testable: 'static {
    type Value: 'static;

    fn is_ok(&self) -> bool;
    fn to_output(self) -> Self::Value;
}

impl Testable for bool {
    type Value = ();

    fn is_ok(&self) -> bool {
        *self
    }

    fn to_output(self) {}
}

impl<T: 'static> Testable for Option<T> {
    type Value = T;

    fn is_ok(&self) -> bool {
        self.is_some()
    }

    fn to_output(self) -> Self::Value {
        self.unwrap()
    }
}

impl<T: 'static, E: 'static> Testable for Result<T, E> {
    type Value = T;

    fn is_ok(&self) -> bool {
        self.is_ok()
    }

    fn to_output(self) -> Self::Value {
        match self {
            Ok(value) => value,
            Err(_) => panic!("Testable::to_output called on Err"),
        }
    }
}
