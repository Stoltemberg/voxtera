use std::{
    collections::VecDeque,
    time::{Duration, Instant},
};

const EMIT_INTERVAL: Duration = Duration::from_millis(250);
const SPEED_WINDOW: Duration = Duration::from_secs(5);

#[derive(Debug, Clone, PartialEq)]
pub struct DownloadProgress {
    pub downloaded_bytes: u64,
    pub total_bytes: u64,
    pub bytes_per_second: Option<f64>,
    pub eta_seconds: Option<f64>,
    pub complete: bool,
}

#[derive(Debug)]
pub struct ProgressThrottle {
    total_bytes: u64,
    last_emit: Option<Instant>,
    samples: VecDeque<(Instant, u64)>,
    emitted_completion: bool,
}

impl ProgressThrottle {
    pub fn new(total_bytes: u64, _started_at: Instant) -> Self {
        Self {
            total_bytes,
            last_emit: None,
            samples: VecDeque::new(),
            emitted_completion: false,
        }
    }

    pub fn observe(
        &mut self,
        downloaded_bytes: u64,
        now: Instant,
        complete: bool,
    ) -> Option<DownloadProgress> {
        if self.emitted_completion {
            return None;
        }
        let ready = self
            .last_emit
            .is_none_or(|last| now.saturating_duration_since(last) >= EMIT_INTERVAL);
        if !ready && !complete {
            return None;
        }

        while self.samples.front().is_some_and(|(sampled_at, _)| {
            now.saturating_duration_since(*sampled_at) > SPEED_WINDOW
        }) {
            self.samples.pop_front();
        }
        self.samples.push_back((now, downloaded_bytes));
        self.last_emit = Some(now);
        self.emitted_completion = complete;

        let bytes_per_second = speed(&self.samples);
        let eta_seconds = if complete {
            None
        } else {
            bytes_per_second
                .filter(|speed| *speed > 0.0)
                .map(|speed| self.total_bytes.saturating_sub(downloaded_bytes) as f64 / speed)
        };

        Some(DownloadProgress {
            downloaded_bytes,
            total_bytes: self.total_bytes,
            bytes_per_second,
            eta_seconds,
            complete,
        })
    }
}

fn speed(samples: &VecDeque<(Instant, u64)>) -> Option<f64> {
    let (first_at, first_bytes) = samples.front()?;
    let (last_at, last_bytes) = samples.back()?;
    if samples.len() < 2 || last_bytes < first_bytes {
        return None;
    }
    let elapsed = last_at.saturating_duration_since(*first_at).as_secs_f64();
    (elapsed > 0.0).then(|| (last_bytes - first_bytes) as f64 / elapsed)
}
