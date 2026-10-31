# Custom CSS support

This fork maintains a downstream Karere variant with optional live CSS injection for the WhatsApp Web content rendered by CEF. The feature is shipped by the Arch User Repository package `karere-custom-css`; it is not part of upstream Karere.

## Location

Karere reads the stylesheet from:

```text
$XDG_CONFIG_HOME/karere/custom.css
```

When `XDG_CONFIG_HOME` is unset, the normal native path is:

```text
~/.config/karere/custom.css
```

Create the directory and file if they do not already exist:

```bash
mkdir -p "${XDG_CONFIG_HOME:-$HOME/.config}/karere"
$EDITOR "${XDG_CONFIG_HOME:-$HOME/.config}/karere/custom.css"
```

## Live reload

`custom.css` is watched while Karere is running. Changes are reapplied automatically to every isolated account browser, so Karere does not need to be restarted. Atomic file replacements performed by editors are supported as well.

The stylesheet is also reapplied after page reloads and when a new WhatsApp Web execution context is created. Removing `custom.css` disables the custom stylesheet and removes the injected style from the page.

## What the CSS affects

The stylesheet is injected into **WhatsApp Web**, not into Karere's GTK/libadwaita interface. Use it to customize the web content displayed inside the application.

Any valid CSS can be used. For example:

```css
/* Example only */
body {
    font-size: 15px;
}
```

WhatsApp Web is not a stable public styling API: its DOM structure, class names, and selectors can change without notice. A rule that works today may therefore need adjustment after a WhatsApp Web update.

Karere's developer tools can be useful for inspecting the current page structure and testing selectors before adding them to `custom.css`.
