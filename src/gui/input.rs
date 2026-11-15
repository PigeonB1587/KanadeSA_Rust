//! Native IME-capable input, using the same asynchronous API as phira.
use std::{
    io,
    sync::mpsc::{self, Receiver},
};

#[derive(Default)]
pub struct NativeInput {
    pending: Option<Receiver<io::Result<Option<String>>>>,
}

impl NativeInput {
    pub fn active(&self) -> bool {
        self.pending.is_some()
    }

    pub fn open(&mut self, text: &str) {
        self.open_with(
            "Search",
            "Search by name in the selected language, or inventory ID",
            text,
        );
    }

    pub fn open_with(&mut self, title: &str, prompt: &str, text: &str) {
        if self.active() {
            return;
        }
        let (tx, rx) = mpsc::channel();
        let result = inputbox::InputBox::new()
            .title(title)
            .prompt(prompt)
            .default_text(text)
            .show_async(move |result| {
                let _ = tx.send(result);
            });
        match result {
            Ok(()) => self.pending = Some(rx),
            Err(error) => super::notifications::error(format!("Cannot open input dialog: {error}")),
        }
    }

    pub fn poll(&mut self) -> Option<String> {
        let result = self.pending.as_ref()?.try_recv();
        match result {
            Err(mpsc::TryRecvError::Empty) => None,
            result => {
                self.pending = None;
                match result {
                    Ok(Ok(text)) => text,
                    Ok(Err(error)) => {
                        super::notifications::error(format!("Input dialog failed: {error}"));
                        None
                    }
                    Err(_) => {
                        super::notifications::error("Input dialog disconnected");
                        None
                    }
                }
            }
        }
    }
}
