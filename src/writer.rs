use tokio::fs::OpenOptions;
use tokio::{io::AsyncWriteExt, sync::mpsc::Receiver};
use tokio_util::sync::CancellationToken;

use std::io::Write;

use crate::merger::MergedSample;

pub struct Writer {
    filename: String,
    input: Receiver<MergedSample>,
    token: CancellationToken,
}

impl Writer {
    pub fn new(
        filename: String,
        input: Receiver<MergedSample>,
        token: CancellationToken,
    ) -> Self {
        Self {
            filename,
            input,
            token,
        }
    }

    pub async fn serialize(&mut self) {
        tokio::io::stdout();
        let mut writer = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&self.filename)
            .await
            .expect("Couldn't open file");

        writer
            .write_all(b"time,sensor 1hz,sensor 100hz\n")
            .await
            .expect("Couldn't write to file");
        let mut buffer = Vec::<u8>::new();

        let mut written = 0;

        while let Some((time, s1, s100)) = self.input.recv().await {
            if self.token.is_cancelled() {
                return;
            }

            writeln!(buffer, "{:.4},{:.4},{:.4}", time, s1, s100)
                .expect("Couldn't write to buffer");
            writer
                .write_all(&buffer)
                .await
                .expect("Couldn't write data to file");

            buffer.clear();
            written += 1;
        }

        println!("samples written: {written}");
    }
}
