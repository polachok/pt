pt
==

pt is a simple tabbed terminal built with gtk-rs and vte-rs.

how to build
------------

You need to have gtk3 glib vte pcre2 dev packages installed on your system.

configuration
-------------

There's no gui configuration, all settings are stored at ~/.config/pterm/config.toml
Default configuration is as follows:

    font_family = "monospace"
    font_size = 11
    [colors]
    foreground = '#ababb2b2bfbf'
    background = '#28272c2c3434'
    palette = [
	"#000000",
	"#e0e06c6b7574",
	"#9898c3c37979",
	"#d1d19a9a6665",
	"#6161afafefef",
	"#c6c67878dddd",
	"#5656b6b6c2c2",
	"#ababb2b2bfbf",
	"#5c5c63637070",
	"#e0e06c6b7574",
	"#9898c3c37979",
	"#d1d19a9a6665",
	"#6161afafefef",
	"#c6c67878dddd",
	"#5656b6b6c2c2",
	"#ffffffffffff",
    ]
