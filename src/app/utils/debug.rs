use tracing::warn;

pub trait SoftExpect {
    fn soft_expect(self, msg: &str) -> Self;
}

// TODO: add automatic component validation for components dependencies that cannot be set to required and improve relationships of component-to-component not entity (i.e. warning if given entity does not contain the desired component)
// TODO: add convenience methods for accessing relationships

impl<T> SoftExpect for Option<T> {
    fn soft_expect(self, msg: &str) -> Self {
        if self.is_none() {
            warn!(
                "{}",
                if msg.is_empty() {
                    "Expected to be Some but was None"
                } else {
                    msg
                }
            );
        }
        self
    }
}
