use std::time::Duration;

use tokio::{sync::mpsc::Sender, time::sleep};
use tokio_util::sync::CancellationToken;

pub type Sample = (usize, f64);

pub struct Sensor<F>
where
    F: Fn(usize) -> f64,
{
    pub frequency: u64,
    fun: F,
    token: CancellationToken,
}

impl<F> Sensor<F>
where
    F: Fn(usize) -> f64,
{
    pub fn new(frequency: u64, fun: F, token: CancellationToken) -> Self {
        Self {
            frequency,
            fun,
            token,
        }
    }

    pub async fn generate(&self, tx: Sender<Sample>) {
        let mut time = 0usize;

        while !self.token.is_cancelled() {
            tx.send((time, (self.fun)(time)))
                .await
                .expect("send failed");

            let delay = 1000 / self.frequency;
            sleep(Duration::from_millis(delay)).await;
            time += delay as usize;
        }
    }
}
