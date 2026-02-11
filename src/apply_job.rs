use std::path::PathBuf;
use std::sync::mpsc;

use crate::monitors::Monitor;
use crate::wallpaper;

enum Msg {
    Status(String),
    Done(Result<(), String>),
}

pub struct ApplyJob {
    rx: Option<mpsc::Receiver<Msg>>,
    status: Option<Result<(), String>>,
    log: String,
}

impl ApplyJob {
    pub fn new() -> Self {
        Self {
            rx: None,
            status: None,
            log: String::new(),
        }
    }

    pub fn start(&mut self, assignments: Vec<(PathBuf, Monitor)>, ctx: &eframe::egui::Context) {
        let (tx, rx) = mpsc::channel();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let log_tx = tx.clone();
            let log_ctx = ctx.clone();
            let log = move |msg: &str| {
                let _ = log_tx.send(Msg::Status(msg.to_string()));
                log_ctx.request_repaint();
            };
            let result = wallpaper::apply(&assignments, &log);
            let _ = tx.send(Msg::Done(result));
            ctx.request_repaint();
        });

        self.rx = Some(rx);
        self.status = None;
    }

    pub fn poll(&mut self) {
        let Some(rx) = &self.rx else { return };
        loop {
            match rx.try_recv() {
                Ok(Msg::Status(msg)) => self.log = msg,
                Ok(Msg::Done(result)) => {
                    self.status = Some(result);
                    self.log.clear();
                    self.rx = None;
                    break;
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.status = Some(Err("apply thread crashed".into()));
                    self.log.clear();
                    self.rx = None;
                    break;
                }
                Err(mpsc::TryRecvError::Empty) => break,
            }
        }
    }

    pub fn is_running(&self) -> bool {
        self.rx.is_some()
    }

    pub fn status(&self) -> Option<&Result<(), String>> {
        self.status.as_ref()
    }

    pub fn log(&self) -> &str {
        &self.log
    }

    pub fn clear_status(&mut self) {
        self.status = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn job_with_rx(rx: mpsc::Receiver<Msg>) -> ApplyJob {
        ApplyJob {
            rx: Some(rx),
            status: None,
            log: String::new(),
        }
    }

    #[test]
    fn new_is_idle() {
        let job = ApplyJob::new();
        assert!(!job.is_running());
        assert!(job.status().is_none());
        assert!(job.log().is_empty());
    }

    #[test]
    fn poll_noop_when_idle() {
        let mut job = ApplyJob::new();
        job.poll();
        assert!(!job.is_running());
    }

    #[test]
    fn poll_status_message() {
        let (tx, rx) = mpsc::channel();
        let mut job = job_with_rx(rx);
        tx.send(Msg::Status("caching...".into())).unwrap();
        job.poll();
        assert_eq!(job.log(), "caching...");
        assert!(job.is_running());
        assert!(job.status().is_none());
    }

    #[test]
    fn poll_done_success() {
        let (tx, rx) = mpsc::channel();
        let mut job = job_with_rx(rx);
        tx.send(Msg::Done(Ok(()))).unwrap();
        job.poll();
        assert!(!job.is_running());
        assert!(job.status().unwrap().is_ok());
        assert!(job.log().is_empty());
    }

    #[test]
    fn poll_done_error() {
        let (tx, rx) = mpsc::channel();
        let mut job = job_with_rx(rx);
        tx.send(Msg::Done(Err("xrandr failed".into()))).unwrap();
        job.poll();
        assert!(!job.is_running());
        assert_eq!(
            job.status().unwrap().as_ref().unwrap_err().as_str(),
            "xrandr failed"
        );
    }

    #[test]
    fn poll_disconnected() {
        let (tx, rx) = mpsc::channel();
        let mut job = job_with_rx(rx);
        drop(tx);
        job.poll();
        assert!(!job.is_running());
        assert!(job.status().unwrap().is_err());
    }

    #[test]
    fn poll_processes_status_then_done() {
        let (tx, rx) = mpsc::channel();
        let mut job = job_with_rx(rx);
        tx.send(Msg::Status("step 1".into())).unwrap();
        tx.send(Msg::Done(Ok(()))).unwrap();
        job.poll();
        // Done consumes the receiver, log is cleared
        assert!(!job.is_running());
        assert!(job.status().unwrap().is_ok());
        assert!(job.log().is_empty());
    }

    #[test]
    fn clear_status() {
        let (tx, rx) = mpsc::channel();
        let mut job = job_with_rx(rx);
        tx.send(Msg::Done(Ok(()))).unwrap();
        job.poll();
        assert!(job.status().is_some());
        job.clear_status();
        assert!(job.status().is_none());
    }
}
