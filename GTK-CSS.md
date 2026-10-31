# Live GTK theme updates

Karere watches CSS files directly inside `$XDG_CONFIG_HOME/gtk-4.0` (normally `~/.config/gtk-4.0`). After a change, it reloads `gtk.css` and its imports with a 150 ms debounce. This updates the native window, headerbar, and dialogs without restarting Karere. Atomic file replacements are supported.

For example, Noctalia can generate `gtk-4.0/noctalia.css`, imported by `gtk-4.0/gtk.css`:

```css
@import url("noctalia.css");
```

This is independent of `karere/custom.css`, which styles WhatsApp Web: the Noctalia integration must generate both the web palette and the GTK palette. Browser CSS must not be loaded into GTK.

The reloadable stylesheet has user priority above application styles, including “Match WhatsApp colors”. GTK also retains its original startup-loaded user stylesheet, which has no public reload API: removing rules may expose their original startup values. Fully removing a theme can therefore still require a restart. Changes to imported files outside the watched directory (including subdirectories or external symlink targets) are not monitored.

## Regression tests

The integration test uses a temporary directory, not the user's configuration. It verifies relative imports and actual GTK color changes after direct writes and atomic replacements.

```bash
GDK_BACKEND=x11 xvfb-run -a cargo test --bin karere gtk_css::tests -- --include-ignored --test-threads=1
```
