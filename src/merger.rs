use tokio::sync::mpsc::Receiver;
use tokio::sync::mpsc::Sender;
use tokio_util::sync::CancellationToken;

use crate::sensor::Sample;

pub type MergedSample = (usize, f64, f64);

pub struct Merger {
    input_slow: Receiver<Sample>,
    input_fast: Receiver<Sample>,
    old_sample: Option<Sample>,
    token: CancellationToken,
}

fn interpolate(
    old_time: usize,
    old_value: f64,
    new_time: usize,
    new_value: f64,
    interp_time: usize,
) -> f64 {
    let time_span = (new_time - old_time) as f64;
    assert!(time_span > 0.0);

    (old_value * (new_time - interp_time) as f64
        + new_value * (interp_time - old_time) as f64)
        / time_span
}

impl Merger {
    pub fn new(
        input_slow: Receiver<Sample>,
        input_fast: Receiver<Sample>,
        token: CancellationToken,
    ) -> Self {
        Self {
            input_slow,
            input_fast,
            old_sample: None,
            token,
        }
    }

    pub async fn process(&mut self, tx: Sender<MergedSample>) {
        while !self.token.is_cancelled() {
            let slow_sample @ Some((slow_time, slow_value)) =
                self.input_slow.recv().await
            else {
                return; // no more samples
            };

            // wait for the first sample of the slow stream to start
            // interpolating
            if self.old_sample.is_none() {
                self.old_sample = slow_sample;
                continue;
            }

            // catch up with the high frequency stream and emit the merged data
            // points
            loop {
                let Some((fast_time, fast_value)) =
                    self.input_fast.recv().await
                else {
                    return; // no more samples
                };

                if let Some((old_time, old_value)) = self.old_sample {
                    let interp = interpolate(
                        old_time, old_value, slow_time, slow_value, fast_time,
                    );

                    tx.send((fast_time, interp, fast_value))
                        .await
                        .expect("send in merge failed")
                }

                if slow_time == fast_time {
                    break;
                }
            }

            self.old_sample = slow_sample;
        }
    }
}

#[test]
fn test_interpolate() {
    // beginning
    assert_eq!(interpolate(0, 0.0, 20, 100.0, 0), 0.0);
    // middle
    assert_eq!(interpolate(0, 0.0, 100, 100.0, 30), 30.0);
    assert_eq!(interpolate(0, 0.0, 100, 100.0, 40), 40.0);
    assert_eq!(interpolate(0, 0.0, 100, 100.0, 50), 50.0);
    //end
    assert_eq!(interpolate(0, 0.0, 30, 30.0, 30), 30.0);
}
