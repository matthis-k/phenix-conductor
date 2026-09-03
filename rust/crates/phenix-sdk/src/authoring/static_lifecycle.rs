#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct StaticPluginLifecycleDescriptor {
    pub start: Option<&'static str>,
    pub stop: Option<&'static str>,
}

pub trait StaticPluginLifecycle {
    fn lifecycle() -> StaticPluginLifecycleDescriptor;
}

impl StaticPluginLifecycleDescriptor {
    #[must_use]
    pub const fn uses_kernel_defaults(self) -> bool {
        self.start.is_none() && self.stop.is_none()
    }
}
