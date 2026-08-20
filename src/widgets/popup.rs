use ratatui::layout::{Constraint, Layout, Rect};

pub fn get_popup_area_centered(rect: Rect, width: u16, height: u16) -> Rect {
    let height = height.min(rect.height);
    let width = width.min(rect.width);
    let top_margin = (rect.height - height) / 2;
    let bottom_margin = rect.height - top_margin - height;
    let left_margin = (rect.width - width) / 2;
    let right_margin = rect.width - left_margin - width;
    get_popup_area(rect, top_margin, right_margin, bottom_margin, left_margin)
}

pub fn get_popup_area(
    rect: Rect,
    top_margin: u16,
    right_margin: u16,
    bottom_margin: u16,
    left_margin: u16,
) -> Rect {
    let chunks = Layout::vertical(vec![
        Constraint::Length(top_margin),
        Constraint::Min(0),
        Constraint::Length(bottom_margin),
    ])
    .split(rect);
    let chunks = Layout::horizontal(vec![
        Constraint::Length(left_margin),
        Constraint::Min(0),
        Constraint::Length(right_margin),
    ])
    .split(chunks[1]);
    chunks[1]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_size_yields_full_rect() {
        let area = Rect::new(5, 6, 20, 10);
        let result = get_popup_area_centered(area, 20, 10);
        assert_eq!(result, area);
    }

    #[test]
    fn oversize_request_is_clamped_to_area() {
        let area = Rect::new(0, 0, 10, 6);
        let result = get_popup_area_centered(area, 50, 50);
        assert_eq!(result, area);
    }

    #[test]
    fn small_popup_is_centered() {
        let area = Rect::new(0, 0, 10, 6);
        // width 4 -> left margin 3 (10 - 4 = 6, /2), top 2 (6 - 2 = 4, /2)
        let result = get_popup_area_centered(area, 4, 2);
        assert_eq!(result, Rect::new(3, 2, 4, 2));
    }

    #[test]
    fn odd_margins_give_one_pillar_to_right_and_bottom() {
        let area = Rect::new(0, 0, 9, 5);
        // width 5 -> 4 leftover -> left 2, right 2
        // height 3 -> 2 leftover -> top 1, bottom 1
        let result = get_popup_area_centered(area, 5, 3);
        assert_eq!(result, Rect::new(2, 1, 5, 3));
        // width 4 -> 5 leftover -> left 2, right 3 (remainder falls to right)
        let result = get_popup_area_centered(area, 4, 3);
        assert_eq!(result, Rect::new(2, 1, 4, 3));
    }

    #[test]
    fn offsets_are_pinned_to_area_origin() {
        let area = Rect::new(10, 20, 30, 20);
        let result = get_popup_area_centered(area, 10, 6);
        assert_eq!(result, Rect::new(10 + 10, 20 + 7, 10, 6));
    }
}
