use tracing::warn;

pub trait SoftExpect {
    fn soft_expect(self, msg: impl Into<String>) -> Self;
}

impl<T> SoftExpect for Option<T> {
    fn soft_expect(self, msg: impl Into<String>) -> Self {
        if self.is_none() {
            let msg = msg.into();
            warn!(
                "{}",
                if msg.is_empty() {
                    "Expected to be Some but was None"
                } else {
                    &msg
                }
            );
        }

        self
    }
}
