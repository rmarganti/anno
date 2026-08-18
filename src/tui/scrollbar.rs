use ratatui::{
    Frame,
    layout::Rect,
    style::Style,
    widgets::{Scrollbar, ScrollbarOrientation, ScrollbarState},
};

/**
 * Convert total content and viewport lengths into Ratatui's position count.
 */
fn scroll_position_count(content_length: usize, viewport_length: usize) -> usize {
    content_length
        .saturating_sub(viewport_length)
        .saturating_add(1)
}

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
    let mut state = ScrollbarState::new(scroll_position_count(content_length, viewport_length))
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
    let mut state = ScrollbarState::new(scroll_position_count(content_length, viewport_length))
        .position(position)
        .viewport_content_length(viewport_length);
    frame.render_stateful_widget(scrollbar, area, &mut state);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn position_count_ends_at_maximum_scroll_offset() {
        let content_length = 400;
        let viewport_length = 87;
        let position_count = scroll_position_count(content_length, viewport_length);

        assert_eq!(position_count - 1, content_length - viewport_length);
    }
}
