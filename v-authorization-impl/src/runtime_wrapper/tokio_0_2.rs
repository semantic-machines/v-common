// RuntimeWrapper for tokio 0.2

pub struct RuntimeWrapper {
    pub runtime: tokio_dep_0_2::runtime::Runtime,
}

impl RuntimeWrapper {
    pub fn new() -> Self {
        let runtime = tokio_dep_0_2::runtime::Runtime::new().unwrap();
        RuntimeWrapper { runtime }
    }

    pub fn version(&self) -> &'static str {
        "tokio 0.2"
    }

    pub fn block_on<F>(&mut self, future: F) -> F::Output
    where
        F: std::future::Future,
    {
        self.runtime.block_on(future)
    }
}

impl Default for RuntimeWrapper {
    fn default() -> Self {
        Self::new()
    }
}
