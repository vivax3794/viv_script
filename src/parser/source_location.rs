#[derive(Debug, Clone, PartialEq, Eq, Copy)]
pub struct SourceLocation {
    pub line_start: usize,
    pub line_end: usize,
    pub char_start: usize,
    pub char_end: usize,
}

impl SourceLocation {
    #[must_use]
    pub fn new(line: usize, char_start: usize, char_end: usize) -> Self {
        Self {
            line_start: line,
            line_end: line,
            char_start,
            char_end,
        }
    }

    #[must_use] pub fn combine(a: &Self, b: &Self) -> Self {
        Self {
            line_start: usize::min(a.line_start, b.line_start),
            line_end: usize::max(a.line_end, b.line_end),
            char_start: usize::min(a.char_start, b.char_start),
            char_end: usize::max(a.char_end, b.char_end),
        }
    }

    #[must_use] pub fn get_line_highlights(&self, source_code: &str) -> String {
        // The source location is based on the source code, so the line should always be found
        // line numbers are also 1-index
        let lines: String = source_code
            .lines()
            .skip(self.line_start - 1)
            .take(self.line_end - self.line_start + 1)
            .enumerate()
            .map(|(index, line)| format!("{} | {}", index + self.line_start, line))
            .collect::<Vec<String>>()
            .join("\n");

        let max_line_number_width: usize = (self.line_start..=self.line_end)
            .into_iter()
            .map(|lin_num| lin_num.to_string().len())
            .max()
            .unwrap();
        let pointer_padding = max_line_number_width + " | ".len() + self.char_start - 1;
        let pointers =
            " ".repeat(pointer_padding) + &"^".repeat(self.char_end - self.char_start + 1);

        format!("{lines}\n{pointers}")
    }
}
