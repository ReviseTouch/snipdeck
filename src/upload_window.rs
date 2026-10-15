//! The upload window's controller: consent, progress and result on the UI
//! thread, the upload itself on a worker (`crate::upload::spawn`).
//!
//! One upload at a time. Asking for another while one runs brings the window
//! back instead of starting a second. Hiding the window during an upload does
//! not cancel it: the result shows the window again, because "Uploaded, the
//! link is on your clipboard" is the one thing the user is waiting to hear.

use crossbeam_channel::{unbounded, Receiver, Sender};
use slint::ComponentHandle;
use std::cell::RefCell;
use std::rc::Rc;

use crate::upload::{Host, Outcome};
use crate::UploadWindow;

const CONFIRM: i32 = 0;
const UPLOADING: i32 = 1;
const DONE: i32 = 2;
const FAILED: i32 = 3;
const SETUP: i32 = 4;

/// What a button asked for, queued by the window's callbacks and acted on in
/// `tick`, outside any Slint callback.
enum Act {
    Upload,
    Close,
    CopyLink,
    OpenLink,
    OpenSettings,
}

pub struct UploadUi {
    window: Option<UploadWindow>,
    acts: Rc<RefCell<Vec<Act>>>,
    /// The image and host being asked about, uploaded, or offered a retry.
    job: Option<(image::RgbaImage, Host)>,
    busy: bool,
    link: String,
    done_tx: Sender<Outcome>,
    done_rx: Receiver<Outcome>,
}

impl UploadUi {
    pub fn new() -> Self {
        let (done_tx, done_rx) = unbounded();
        Self {
            window: None,
            acts: Rc::new(RefCell::new(Vec::new())),
            job: None,
            busy: false,
            link: String::new(),
            done_tx,
            done_rx,
        }
    }

    /// Ask to upload `img` to `host`. Nothing is sent until the user says so.
    pub fn start(&mut self, img: image::RgbaImage, host: Host, dark: bool) -> Result<(), slint::PlatformError> {
        let w = self.window(dark)?;
        if self.busy {
            w.show()?;
            crate::window_env::keep_above(&w);
            return Ok(());
        }
        if host.needs_client_id() {
            self.job = None;
            w.set_settings_path(crate::paths::settings_file().display().to_string().into());
            w.set_state(SETUP);
        } else {
            w.set_host_name(host.name().into());
            w.set_host_domain(host.domain().into());
            w.set_unlisted(host.is_unlisted());
            w.set_permanent(host.is_permanent());
            w.set_encrypted(host.is_encrypted());
            w.set_state(CONFIRM);
            self.job = Some((img, host));
        }
        // Centred on the monitor under the pointer, as the collage editor is.
        // A Wayland compositor centres a new window by itself and ignores
        // this; Windows put it in the corner of the screen, over the gallery.
        if let Some((px, py)) = crate::screen::centre_on_cursor_monitor(440.0, 300.0) {
            w.window().set_position(slint::PhysicalPosition::new(px, py));
        }
        w.show()?;
        crate::window_env::keep_above(&w);
        w.window().request_redraw();
        Ok(())
    }

    /// Act on the buttons and on a finished upload. Returns true if anything
    /// happened, to keep the pump awake for the next step.
    pub fn tick(&mut self) -> bool {
        let acts: Vec<Act> = self.acts.borrow_mut().drain(..).collect();
        let mut worked = !acts.is_empty();
        for act in acts {
            match act {
                Act::Upload => self.upload(),
                Act::Close => {
                    if let Some(w) = &self.window {
                        let _ = w.hide();
                    }
                    if !self.busy {
                        self.job = None;
                    }
                }
                Act::CopyLink => {
                    let copied = crate::clipboard::copy_text(&self.link).is_ok();
                    if let Some(w) = &self.window {
                        w.set_copied(copied);
                    }
                }
                Act::OpenLink => crate::share::open_url(&self.link),
                Act::OpenSettings => {
                    crate::share::open_url(&crate::paths::settings_file().display().to_string())
                }
            }
        }
        for outcome in self.done_rx.try_iter() {
            worked = true;
            self.busy = false;
            let Some(w) = &self.window else { continue };
            match outcome {
                Outcome::Done { url, copied } => {
                    w.set_link(url.as_str().into());
                    w.set_copied(copied);
                    w.set_state(DONE);
                    self.link = url;
                    self.job = None;
                }
                Outcome::Failed(why) => {
                    w.set_error_text(why.into());
                    w.set_state(FAILED);
                }
            }
            let _ = w.show();
            crate::window_env::keep_above(w);
        }
        worked
    }

    fn upload(&mut self) {
        if self.busy {
            return;
        }
        let Some((img, host)) = &self.job else { return };
        self.busy = true;
        if let Some(w) = &self.window {
            w.set_state(UPLOADING);
        }
        crate::upload::spawn(img.clone(), host.clone(), self.done_tx.clone());
    }

    fn window(&mut self, dark: bool) -> Result<UploadWindow, slint::PlatformError> {
        if self.window.is_none() {
            let w = UploadWindow::new()?;
            let queue = |acts: &Rc<RefCell<Vec<Act>>>, act: fn() -> Act| {
                let acts = acts.clone();
                move || {
                    acts.borrow_mut().push(act());
                    crate::pump::wake();
                }
            };
            w.on_upload(queue(&self.acts, || Act::Upload));
            w.on_close(queue(&self.acts, || Act::Close));
            w.on_copy_link(queue(&self.acts, || Act::CopyLink));
            w.on_open_link(queue(&self.acts, || Act::OpenLink));
            w.on_open_settings(queue(&self.acts, || Act::OpenSettings));
            self.window = Some(w);
        }
        let w = self.window.as_ref().expect("just created").clone_strong();
        w.set_dark(dark);
        Ok(w)
    }
}
