use web_sys::window;

pub fn get_inner_window_dimensions() -> Option<(usize, usize)> {
    let window = window()?;
    let inner_height = window.inner_height().ok()?.as_f64()?;
    let inner_width = window.inner_width().ok()?.as_f64()?;

    Some((inner_height as usize, inner_width as usize))
}