# Linux toolkit
A toolkit for building Linux GUI applications. Handles common wayland and dbus
protocols, implements XDG specifications and provides utilities for locale
handling and locating system fonts. It is not a widget or rendering library.

## Features
* Multiple monitors with different DPI scale factors
* Multiseat setups
* Handles multiple surfaces
* Uses xkbcommon to load the keyboard map and supports key repeating
* DPI scaleable cursor and cursor theme loading
* System clipboard handling
* Supports the xdg-shell and the layer-shell
* Locale detection

## Features not implemented yet
* xdg base directories
* desktop file generation from Cargo.toml
* appdata.xml generation from Cargo.toml
* locating system fonts
* app id parsing
* dbusmenu protocol implementation
* dbus notifier item protocol implementation
* utilities for creating egl and wsi surfaces
* window decorations

# License
Copyright 2019 David Craven

Permission to use, copy, modify, and/or distribute this software for any purpose
with or without fee is hereby granted, provided that the above copyright notice
and this permission notice appear in all copies.

THE SOFTWARE IS PROVIDED "AS IS" AND THE AUTHOR DISCLAIMS ALL WARRANTIES WITH
REGARD TO THIS SOFTWARE INCLUDING ALL IMPLIED WARRANTIES OF MERCHANTABILITY AND
FITNESS. IN NO EVENT SHALL THE AUTHOR BE LIABLE FOR ANY SPECIAL, DIRECT,
INDIRECT, OR CONSEQUENTIAL DAMAGES OR ANY DAMAGES WHATSOEVER RESULTING FROM LOSS
OF USE, DATA OR PROFITS, WHETHER IN AN ACTION OF CONTRACT, NEGLIGENCE OR OTHER
TORTIOUS ACTION, ARISING OUT OF OR IN CONNECTION WITH THE USE OR PERFORMANCE OF
THIS SOFTWARE.
