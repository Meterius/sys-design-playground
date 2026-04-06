use bevy::log::warn;

pub trait SoftExpect {
    fn soft_expect(self, msg: &str) -> Self;
}

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
