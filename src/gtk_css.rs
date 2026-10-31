//! Live reload of the user's GTK stylesheet (separate from web custom CSS).
use std::cell::RefCell;
use std::path::Path;
use std::rc::Rc;
use std::time::Duration;

use gtk::{gio, glib, prelude::*};

pub(crate) struct UserCss {
    provider: gtk::CssProvider,
    display: gtk::gdk::Display,
    file: gio::File,
    monitor: gio::FileMonitor,
    pending: RefCell<Option<glib::SourceId>>,
}

fn relevant_event(
    file: &gio::File,
    other: Option<&gio::File>,
    event: gio::FileMonitorEvent,
) -> bool {
    matches!(
        event,
        gio::FileMonitorEvent::Changed
            | gio::FileMonitorEvent::ChangesDoneHint
            | gio::FileMonitorEvent::Created
            | gio::FileMonitorEvent::Deleted
            | gio::FileMonitorEvent::AttributeChanged
            | gio::FileMonitorEvent::Moved
            | gio::FileMonitorEvent::Renamed
            | gio::FileMonitorEvent::MovedIn
            | gio::FileMonitorEvent::MovedOut
    ) && std::iter::once(file).chain(other).any(|file| {
        file.basename()
            .is_some_and(|name| Path::new(&name).extension().is_some_and(|ext| ext == "css"))
    })
}

impl UserCss {
    pub(crate) fn start() -> Option<Rc<Self>> {
        let display = gtk::gdk::Display::default()?;
        let directory = glib::user_config_dir().join("gtk-4.0");
        Self::start_in(display, &directory)
    }

    fn start_in(display: gtk::gdk::Display, directory: &Path) -> Option<Rc<Self>> {
        if let Err(error) = std::fs::create_dir_all(directory) {
            log::warn!(
                "cannot create GTK CSS directory {}: {error}",
                directory.display()
            );
            return None;
        }
        let monitor = match gio::File::for_path(directory)
            .monitor_directory(gio::FileMonitorFlags::WATCH_MOVES, gio::Cancellable::NONE)
        {
            Ok(monitor) => monitor,
            Err(error) => {
                log::warn!(
                    "cannot monitor GTK CSS directory {}: {error}",
                    directory.display()
                );
                return None;
            }
        };
        let provider = gtk::CssProvider::new();
        provider.connect_parsing_error(|_, _, error| {
            log::warn!("user GTK CSS: {error}");
        });
        // GTK's startup-loaded user provider is not exposed publicly. Place our
        // reloadable copy above it, without modifying the user's files/settings.
        gtk::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_USER + 1,
        );
        let state = Rc::new(Self {
            provider,
            display,
            file: gio::File::for_path(directory.join("gtk.css")),
            monitor,
            pending: RefCell::new(None),
        });
        let weak = Rc::downgrade(&state);
        state.monitor.connect_changed(move |_, file, other, event| {
            if !relevant_event(file, other, event) {
                return;
            }
            let Some(state) = weak.upgrade() else { return };
            if let Some(source) = state.pending.borrow_mut().take() {
                source.remove();
            }
            let weak = Rc::downgrade(&state);
            let source = glib::timeout_add_local_once(Duration::from_millis(150), move || {
                if let Some(state) = weak.upgrade() {
                    state.pending.borrow_mut().take();
                    state.reload();
                }
            });
            *state.pending.borrow_mut() = Some(source);
        });
        state.reload();
        Some(state)
    }

    fn reload(&self) {
        if self.file.query_exists(gio::Cancellable::NONE) {
            // Loading from a file preserves the base URI for relative @imports.
            self.provider.load_from_file(&self.file);
        } else {
            self.provider.load_from_string("");
        }
        log::debug!("reloaded user GTK stylesheet");
    }
}

impl Drop for UserCss {
    fn drop(&mut self) {
        self.monitor.cancel();
        if let Some(source) = self.pending.get_mut().take() {
            source.remove();
        }
        gtk::style_context_remove_provider_for_display(&self.display, &self.provider);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "requires a GTK display (run with xvfb-run)"]
    fn reloads_relative_import_after_write_and_atomic_replace() {
        gtk::init().unwrap();
        let directory = std::env::temp_dir().join(format!("karere-gtk-css-{}", std::process::id()));
        std::fs::create_dir_all(&directory).unwrap();
        std::fs::write(directory.join("gtk.css"), "@import url(\"noctalia.css\");").unwrap();
        let imported = directory.join("noctalia.css");
        std::fs::write(&imported, "@define-color karere_live_test #ff0000;").unwrap();
        let state = UserCss::start_in(gtk::gdk::Display::default().unwrap(), &directory).unwrap();
        let label = gtk::Label::new(None);
        let color = || {
            #[allow(deprecated)]
            label
                .style_context()
                .lookup_color("karere_live_test")
                .unwrap()
        };
        assert_eq!(color(), gtk::gdk::RGBA::RED);
        let wait_for = |expected| {
            let deadline = std::time::Instant::now() + Duration::from_secs(3);
            while color() != expected {
                assert!(
                    std::time::Instant::now() < deadline,
                    "GTK CSS did not reload"
                );
                while glib::MainContext::default().iteration(false) {}
                std::thread::sleep(Duration::from_millis(10));
            }
        };
        std::fs::write(&imported, "@define-color karere_live_test #0000ff;").unwrap();
        wait_for(gtk::gdk::RGBA::BLUE);
        let replacement = directory.join(".replacement");
        std::fs::write(&replacement, "@define-color karere_live_test #ff0000;").unwrap();
        std::fs::rename(replacement, &imported).unwrap();
        wait_for(gtk::gdk::RGBA::RED);
        drop(state);
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn watches_entrypoint_and_imports() {
        for name in ["gtk.css", "noctalia.css", "noctalia-extensions.css"] {
            let file = gio::File::for_path(format!("/config/gtk-4.0/{name}"));
            for event in [
                gio::FileMonitorEvent::Changed,
                gio::FileMonitorEvent::ChangesDoneHint,
                gio::FileMonitorEvent::Created,
                gio::FileMonitorEvent::Deleted,
                gio::FileMonitorEvent::MovedIn,
                gio::FileMonitorEvent::MovedOut,
            ] {
                assert!(relevant_event(&file, None, event));
            }
        }
    }

    #[test]
    fn atomic_replacement_checks_both_paths() {
        let temp = gio::File::for_path("/config/gtk-4.0/.tmp123");
        let css = gio::File::for_path("/config/gtk-4.0/noctalia.css");
        assert!(relevant_event(
            &temp,
            Some(&css),
            gio::FileMonitorEvent::Renamed
        ));
        assert!(relevant_event(
            &css,
            Some(&temp),
            gio::FileMonitorEvent::Renamed
        ));
        assert!(!relevant_event(&temp, None, gio::FileMonitorEvent::Changed));
        assert!(!relevant_event(
            &css,
            None,
            gio::FileMonitorEvent::Unmounted
        ));
    }
}
