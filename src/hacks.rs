use glib::translate::*;
use gtk::prelude::*;

// TODO: in gtk master they have it, remove when updating
pub fn parse_color(s: &str) -> Result<gdk::RGBA, glib::error::BoolError> {
    use glib::translate::*;
    unsafe {
        let mut res = gdk::RGBA {
            alpha: 0.0,
            blue: 0.0,
            green: 0.0,
            red: 0.0,
        };
        glib::result_from_gboolean!(
            gdk_sys::gdk_rgba_parse(res.to_glib_none_mut().0, s.to_glib_none().0),
            "Can't parse RGBA"
        )
        .map(|_| res)
    }
}

pub fn set_child_property(
    container: impl IsA<gtk::Container>,
    child: impl IsA<gtk::Widget>,
    property_name: &str,
    value: impl ToValue,
) {
    let container = container.upcast();
    let child = child.upcast();

    unsafe {
        gtk_sys::gtk_container_child_set_property(
            container.to_glib_none().0,
            child.to_glib_none().0,
            property_name.to_glib_none().0,
            &value.to_value().into_raw(),
        )
    }
}
