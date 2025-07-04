pub struct RuntimeWrapper {
    pub runtime: tokio_dep_1::runtime::Runtime,
}

impl RuntimeWrapper {
    pub fn new() -> Self {
        let runtime = tokio_dep_1::runtime::Runtime::new().unwrap();
        RuntimeWrapper {
            runtime,
        }
    }

    pub fn version(&self) -> &'static str {
        "tokio 1.0"
    }

    pub fn block_on<F>(&mut self, future: F) -> F::Output
    where
        F: std::future::Future,
    {
        self.runtime.block_on(future)
    }
}
