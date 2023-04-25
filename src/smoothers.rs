pub trait Smoother {
    fn smooth(&self, samples: Vec<i16>) -> Vec<i16>;
}

pub struct NoSmoother {}

impl NoSmoother {
    pub fn new() -> Self {
        Self {}
    }
}

impl Smoother for NoSmoother {
    fn smooth(&self, samples: Vec<i16>) -> Vec<i16> {
        samples
    }
}
