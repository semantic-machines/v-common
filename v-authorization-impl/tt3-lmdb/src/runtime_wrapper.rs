// RuntimeWrapper for tokio 1.x

pub struct RuntimeWrapper {
    pub runtime: tokio::runtime::Runtime,
}

impl RuntimeWrapper {
    pub fn new() -> Self {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        RuntimeWrapper { runtime }
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
