use ratatui::{
    Frame,
    layout::Rect,
    style::Style,
    widgets::{Scrollbar, ScrollbarOrientation, ScrollbarState},
};

/**
 * Render a vertical scrollbar when the content exceeds the viewport.
 */
pub fn render_vertical_scrollbar(
    frame: &mut Frame,
    area: Rect,
    position: usize,
    content_length: usize,
    viewport_length: usize,
    style: Style,
) {
    if area.width == 0 || area.height == 0 || content_length <= viewport_length {
        return;
    }

    let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
        .begin_symbol(None)
        .end_symbol(None)
        .thumb_symbol("▐")
        .track_symbol(None)
        .thumb_style(style);
    let mut state = ScrollbarState::new(content_length)
        .position(position)
        .viewport_content_length(viewport_length);
    frame.render_stateful_widget(scrollbar, area, &mut state);
}

/**
 * Render a horizontal scrollbar when the content exceeds the viewport.
 */
pub fn render_horizontal_scrollbar(
    frame: &mut Frame,
    area: Rect,
    position: usize,
    content_length: usize,
    viewport_length: usize,
    style: Style,
) {
    if area.width == 0 || area.height == 0 || content_length <= viewport_length {
        return;
    }

    let scrollbar = Scrollbar::new(ScrollbarOrientation::HorizontalBottom)
        .begin_symbol(None)
        .end_symbol(None)
        .thumb_symbol("▂")
        .track_symbol(None)
        .thumb_style(style);
    let mut state = ScrollbarState::new(content_length)
        .position(position)
        .viewport_content_length(viewport_length);
    frame.render_stateful_widget(scrollbar, area, &mut state);
}
