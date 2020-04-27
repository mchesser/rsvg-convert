# rsvg-convert

A lightweight wrapper around Inkscape, that translates `rsvg-convert` CLI options to Inkscape options. Also caches outputs to a temporary location.

Designed for use with `pandoc` for automatically converting SVG files when targeting PDF outputs on platforms where `rsvg-convert` is difficult to install (i.e. Windows).

