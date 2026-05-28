#![expect(clippy::expect_used)]

use crate::test_backend::VT100Backend;
use ratatui::layout::Rect;
use ratatui::style::Stylize;
use ratatui::text::Line;

// Small helper macro to assert a collection contains an item with a clearer
// failure message.
macro_rules! assert_contains {
    ($collection:expr, $item:expr $(,)?) => {
        assert!(
            $collection.contains(&$item),
            "Expected {:?} to contain {:?}",
            $collection,
            $item
        );
    };
    ($collection:expr, $item:expr, $($arg:tt)+) => {
        assert!($collection.contains(&$item), $($arg)+);
    };
}

struct TestScenario {
    term: codex_tui::Terminal<VT100Backend>,
}

impl TestScenario {
    fn new(width: u16, height: u16, viewport: Rect) -> Self {
        let backend = VT100Backend::new(width, height);
        let mut term =
            codex_tui::Terminal::with_options(backend).expect("failed to construct terminal");
        term.set_viewport_area(viewport);
        Self { term }
    }

    fn run_insert(&mut self, lines: Vec<Line<'static>>) {
        codex_tui::insert_history_lines(&mut self.term, lines)
            .expect("Failed to insert history lines in test");
    }
}

#[test]
fn basic_insertion_no_wrap() {
    // Screen of 20x6; viewport is the last row (height=1 at y=5)
    let area = Rect::new(
        /*x*/ 0, /*y*/ 5, /*width*/ 20, /*height*/ 1,
    );
    let mut scenario = TestScenario::new(/*width*/ 20, /*height*/ 6, area);

    let lines = vec!["first".into(), "second".into()];
    scenario.run_insert(lines);
    let rows = scenario.term.backend().vt100().screen().contents();
    assert_contains!(rows, String::from("first"));
    assert_contains!(rows, String::from("second"));
}

#[test]
fn long_token_wraps() {
    let area = Rect::new(
        /*x*/ 0, /*y*/ 5, /*width*/ 20, /*height*/ 1,
    );
    let mut scenario = TestScenario::new(/*width*/ 20, /*height*/ 6, area);

    let long = "A".repeat(45); // > 2 lines at width 20
    let lines = vec![long.clone().into()];
    scenario.run_insert(lines);
    let screen = scenario.term.backend().vt100().screen();

    // Count total A's on the screen
    let mut count_a = 0usize;
    for row in 0..6 {
        for col in 0..20 {
            if let Some(cell) = screen.cell(row, col)
                && let Some(ch) = cell.contents().chars().next()
                && ch == 'A'
            {
                count_a += 1;
            }
        }
    }

    assert_eq!(
        count_a,
        long.len(),
        "wrapped content did not preserve all characters"
    );
}

#[test]
fn emoji_and_cjk() {
    let area = Rect::new(
        /*x*/ 0, /*y*/ 5, /*width*/ 20, /*height*/ 1,
    );
    let mut scenario = TestScenario::new(/*width*/ 20, /*height*/ 6, area);

    let text = String::from("😀😀😀😀😀 你好世界");
    let lines = vec![text.clone().into()];
    scenario.run_insert(lines);
    let rows = scenario.term.backend().vt100().screen().contents();
    for ch in text.chars().filter(|c| !c.is_whitespace()) {
        assert!(
            rows.contains(ch),
            "missing character {ch:?} in reconstructed screen"
        );
    }
}

#[test]
fn mixed_ansi_spans() {
    let area = Rect::new(
        /*x*/ 0, /*y*/ 5, /*width*/ 20, /*height*/ 1,
    );
    let mut scenario = TestScenario::new(/*width*/ 20, /*height*/ 6, area);

    let line = vec!["red".red(), "+plain".into()].into();
    scenario.run_insert(vec![line]);
    let rows = scenario.term.backend().vt100().screen().contents();
    assert_contains!(rows, String::from("red+plain"));
}

#[test]
fn cursor_restoration() {
    let area = Rect::new(
        /*x*/ 0, /*y*/ 5, /*width*/ 20, /*height*/ 1,
    );
    let mut scenario = TestScenario::new(/*width*/ 20, /*height*/ 6, area);

    let lines = vec!["x".into()];
    scenario.run_insert(lines);
    assert_eq!(scenario.term.last_known_cursor_pos, (0, 0).into());
}

#[test]
fn word_wrap_no_mid_word_split() {
    // Screen of 40x10; viewport is the last row
    let area = Rect::new(
        /*x*/ 0, /*y*/ 9, /*width*/ 40, /*height*/ 1,
    );
    let mut scenario = TestScenario::new(/*width*/ 40, /*height*/ 10, area);

    let sample = "Years passed, and Willowmere thrived in peace and friendship. Mira’s herb garden flourished with both ordinary and enchanted plants, and travelers spoke of the kindness of the woman who tended them.";
    scenario.run_insert(vec![sample.into()]);
    let joined = scenario.term.backend().vt100().screen().contents();
    assert!(
        !joined.contains("bo\nth"),
        "word 'both' should not be split across lines:\n{joined}"
    );
}

#[test]
fn em_dash_and_space_word_wrap() {
    // Repro from report: ensure we break before "inside", not mid-word.
    let area = Rect::new(
        /*x*/ 0, /*y*/ 9, /*width*/ 40, /*height*/ 1,
    );
    let mut scenario = TestScenario::new(/*width*/ 40, /*height*/ 10, area);

    let sample = "Mara found an old key on the shore. Curious, she opened a tarnished box half-buried in sand—and inside lay a single, glowing seed.";
    scenario.run_insert(vec![sample.into()]);
    let joined = scenario.term.backend().vt100().screen().contents();
    assert!(
        !joined.contains("insi\nde"),
        "word 'inside' should not be split across lines:\n{joined}"
    );
}

#[test]
fn incremental_stream_history_keeps_all_numbered_lines() {
    // Match the real chat behavior: small inline viewport at the bottom and
    // one history insertion per commit tick.
    let area = Rect::new(
        /*x*/ 0, /*y*/ 24, /*width*/ 120, /*height*/ 1,
    );
    let mut scenario = TestScenario::new(/*width*/ 120, /*height*/ 25, area);

    for i in 1..=40 {
        scenario.run_insert(vec![format!("FILLER-{i:04}").into()]);
    }
    for i in 1..=400 {
        scenario.run_insert(vec![format!("S-{i:04}").into()]);
    }

    let rows = scenario.term.backend().vt100().screen().contents();
    let mut seen = Vec::new();
    for line in rows.lines() {
        if let Some(rest) = line.strip_prefix("S-")
            && rest.len() == 4
            && rest.chars().all(|ch| ch.is_ascii_digit())
        {
            seen.push(rest.parse::<u16>().expect("valid S-#### suffix"));
        }
    }

    // A vt100 screen model only retains the visible area. We only assert that the
    // visible S-lines form a contiguous tail with no missing mid-sequence rows.
    let mut sorted = seen.clone();
    sorted.sort_unstable();
    sorted.dedup();
    assert!(
        sorted.len() >= 20,
        "expected to retain a visible tail of S-lines, got {sorted:?}\nrows:\n{rows}"
    );
    let expected_start = sorted[0];
    let expected_end = *sorted.last().expect("non-empty");
    let expected_len = (expected_end - expected_start + 1) as usize;
    assert_eq!(
        sorted.len(),
        expected_len,
        "expected contiguous visible S-line tail but found a gap: {sorted:?}\nrows:\n{rows}"
    );
}
