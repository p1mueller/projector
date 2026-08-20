//! Tracks the cursor and the visible window over a piece of content.
//!
//! `ScrollState` stores where a scrolling window starts (`start_position`)
//! and where, within it, the cursor or selection sits (`selected_position`),
//! plus how many items the content currently holds and how many the viewport
//! can show. It is `Copy` so it can be shared freely.

/// The cursor/window position over a fixed piece of scrollable content.
///
/// The absolute cursor position is always `start_position() + selected_position()`,
/// with `selected_position()` constrained to the viewport.
#[derive(Debug, Default, Clone, Copy)]
pub struct ScrollState {
    /// Number of items in the content.
    content_length: usize,
    /// First content index currently shown in the viewport.
    start_position: usize,
    /// Cursor offset within the viewport (0..viewport).
    selected_position: usize,
    /// How many items the viewport can display at once.
    viewport_length: usize,
}

impl ScrollState {
    /// First content index currently visible in the viewport.
    pub fn start_position(&self) -> usize {
        self.start_position
    }

    /// Cursor/selection offset within the viewport.
    pub fn selected_position(&self) -> usize {
        self.selected_position
    }

    /// How many items the viewport displays at once.
    pub fn viewport_length(&self) -> usize {
        self.viewport_length
    }

    /// Set the content size, clamping the cursor onto the last item if it now
    /// overflows the (possibly shorter) content.
    pub fn set_content_length(&mut self, content_length: usize) {
        self.content_length = content_length;
        if self.start_position + self.selected_position >= content_length {
            self.last();
        }
    }

    /// Set the viewport size, re-centering the window and clamping the
    /// selection so it stays inside the (possibly smaller) viewport.
    pub fn set_viewport_length(&mut self, viewport_length: usize) {
        self.viewport_length = viewport_length;
        let excess_blanks = (self.start_position + self.viewport_length)
            .saturating_sub(self.content_length)
            .min(self.start_position);
        if excess_blanks > 0 {
            self.start_position -= excess_blanks;
            self.selected_position += excess_blanks;
        }
        if self.selected_position >= viewport_length {
            self.selected_position = viewport_length.saturating_sub(1);
        }
    }

    /// Advance the cursor one step, scrolling the window once the cursor
    /// reaches the viewport's end. No-op at the end of the content.
    pub fn next(&mut self) {
        if !self.is_at_end() && self.is_at_viewport_end() {
            self.start_position += 1;
        } else if !self.is_at_end() && !self.is_at_viewport_end() {
            self.selected_position += 1;
        }
    }

    /// Move the cursor back one step, scrolling the window up once the cursor
    /// reaches the viewport's start. No-op at the beginning of the content.
    pub fn prev(&mut self) {
        if !self.is_at_beginning() && self.is_at_viewport_start() {
            self.start_position -= 1;
        } else if !self.is_at_beginning() && !self.is_at_viewport_start() {
            self.selected_position -= 1;
        }
    }

    /// Set the content size and reset the cursor and window to the start.
    pub fn reset_content(&mut self, content_length: usize) {
        self.content_length = content_length;
        self.start_position = 0;
        self.selected_position = 0;
    }

    /// Jump the cursor to the last item, scrolling the window to the bottom.
    pub fn last(&mut self) {
        let effective_viewport_length = usize::min(self.viewport_length, self.content_length);
        self.start_position = self.content_length - effective_viewport_length;
        self.selected_position = effective_viewport_length.saturating_sub(1);
    }

    // Whether the cursor is at the end of the current viewport window.
    fn is_at_viewport_end(&self) -> bool {
        self.selected_position == self.viewport_length.saturating_sub(1)
    }

    // Whether the cursor is at the start of the current viewport window.
    fn is_at_viewport_start(&self) -> bool {
        self.selected_position == 0
    }

    // Whether the cursor is on the very last content item.
    fn is_at_end(&self) -> bool {
        self.start_position + self.selected_position == self.content_length.saturating_sub(1)
    }

    // Whether the cursor is on the very first content item.
    fn is_at_beginning(&self) -> bool {
        self.start_position == 0 && self.selected_position == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make(content: usize, viewport: usize) -> ScrollState {
        let mut s = ScrollState::default();
        s.set_content_length(content);
        s.set_viewport_length(viewport);
        s
    }

    fn cursor(s: &ScrollState) -> usize {
        s.start_position() + s.selected_position()
    }

    fn advance_to_end(s: &mut ScrollState, content: usize) {
        while cursor(s) < content.saturating_sub(1) {
            s.next();
        }
    }

    #[test]
    fn next_advances_within_viewport_then_scrolls() {
        let mut s = make(10, 4);
        assert_eq!((s.start_position(), s.selected_position()), (0, 0));
        s.next();
        assert_eq!((s.start_position(), s.selected_position()), (0, 1));
        s.next();
        assert_eq!((s.start_position(), s.selected_position()), (0, 2));
        s.next();
        assert_eq!((s.start_position(), s.selected_position()), (0, 3));
        // At the viewport end: start scrolling the window.
        s.next();
        assert_eq!((s.start_position(), s.selected_position()), (1, 3));
        s.next();
        assert_eq!((s.start_position(), s.selected_position()), (2, 3));
    }

    #[test]
    fn next_stops_at_end_of_content() {
        let mut s = make(10, 4);
        advance_to_end(&mut s, 10);
        assert_eq!((s.start_position(), s.selected_position()), (6, 3));
        // Already at the end: further advances are a no-op.
        s.next();
        s.next();
        assert_eq!((s.start_position(), s.selected_position()), (6, 3));
    }

    #[test]
    fn prev_winds_back_to_beginning() {
        let mut s = make(10, 4);
        advance_to_end(&mut s, 10);
        let mut expected = (6usize, 3usize);
        while expected.0 > 0 || expected.1 > 0 {
            expected = if expected.1 > 0 {
                (expected.0, expected.1 - 1)
            } else {
                (expected.0 - 1, expected.1)
            };
            s.prev();
            assert_eq!((s.start_position(), s.selected_position()), expected);
        }
        // At the beginning: further retreats are a no-op.
        s.prev();
        assert_eq!((s.start_position(), s.selected_position()), (0, 0));
    }

    #[test]
    fn next_prev_are_inverse() {
        let mut s = make(17, 5);
        for _ in 0..10 {
            s.next();
        }
        assert_eq!(cursor(&s), 10);
        for _ in 0..10 {
            s.prev();
        }
        assert_eq!((s.start_position(), s.selected_position()), (0, 0));
    }

    #[test]
    fn last_jumps_to_end_and_respects_small_content() {
        let mut s = make(10, 4);
        s.last();
        assert_eq!((s.start_position(), s.selected_position()), (6, 3));
        assert_eq!(cursor(&s), 9);

        let mut small = make(2, 4);
        small.last();
        assert_eq!((small.start_position(), small.selected_position()), (0, 1));
        assert_eq!(cursor(&small), 1);
    }

    #[test]
    fn shrinking_content_clamps_cursor() {
        let mut s = make(10, 4);
        advance_to_end(&mut s, 10);
        s.set_content_length(5);
        // Cursor is clamped onto the last character (index 4).
        assert_eq!((s.start_position(), s.selected_position()), (1, 3));
        assert_eq!(cursor(&s), 4);
    }

    #[test]
    fn growing_content_keeps_cursor() {
        let mut s = make(5, 4);
        advance_to_end(&mut s, 5);
        assert_eq!((s.start_position(), s.selected_position()), (1, 3));
        s.set_content_length(20);
        assert_eq!((s.start_position(), s.selected_position()), (1, 3));
        assert_eq!(cursor(&s), 4);
    }

    #[test]
    fn enlarging_viewport_keeps_whole_content_visible() {
        let mut s = make(10, 4);
        advance_to_end(&mut s, 10);
        s.set_viewport_length(20);
        // The entire content fits: window returns to the start with the cursor on the last char.
        assert_eq!((s.start_position(), s.selected_position()), (0, 9));
        assert_eq!(cursor(&s), 9);
    }

    #[test]
    fn shrinking_viewport_clamps_selection() {
        let mut s = make(10, 0);
        s.set_viewport_length(20);
        advance_to_end(&mut s, 10);
        s.set_viewport_length(4);
        assert!(s.selected_position() < 4);
        assert!(cursor(&s) < 10);
    }

    #[test]
    fn zero_viewport_does_not_underflow() {
        let mut s = make(5, 4);
        s.next();
        s.next();
        // Regression: `selected_position = viewport_length - 1` used to panic for a zero viewport.
        s.set_viewport_length(0);
        assert_eq!(s.selected_position(), 0);
    }

    #[test]
    fn reset_content_clears_positions() {
        let mut s = make(10, 4);
        advance_to_end(&mut s, 10);
        assert!(cursor(&s) > 0);
        s.reset_content(12);
        assert_eq!((s.start_position(), s.selected_position()), (0, 0));
    }

    #[test]
    fn empty_state_does_not_panic() {
        let mut s = ScrollState::default();
        s.set_content_length(0);
        s.set_viewport_length(0);
        s.next();
        s.prev();
        s.last();
        assert_eq!((s.start_position(), s.selected_position()), (0, 0));
    }
}
