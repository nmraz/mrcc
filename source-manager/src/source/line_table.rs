use std::vec::Vec;

use crate::pos::LineCol;

pub struct LineTable {
    line_offsets: Vec<u32>,
}

impl LineTable {
    pub fn new_for_src(src: &str) -> Self {
        let mut line_offsets = vec![0];

        for (off, c) in src.char_indices() {
            if c == '\n' {
                line_offsets.push((off + 1) as u32);
            }
        }

        LineTable { line_offsets }
    }

    pub fn get_linecol(&self, off: u32) -> LineCol {
        let line = self
            .line_offsets
            .binary_search(&off)
            .unwrap_or_else(|i| i - 1);

        let col = off - self.line_offsets[line];

        LineCol {
            line: line as u32,
            col,
        }
    }

    pub fn line_count(&self) -> u32 {
        self.line_offsets.len() as u32
    }

    pub fn get_line_start(&self, line: u32) -> u32 {
        self.line_offsets[line as usize]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_line_table() -> LineTable {
        let src = "Test\nline\n\n  other line!";
        LineTable::new_for_src(src)
    }

    #[test]
    fn lookup() {
        let table = create_line_table();
        assert_eq!(table.get_linecol(0), LineCol { line: 0, col: 0 });
        assert_eq!(table.get_linecol(2), LineCol { line: 0, col: 2 });
        assert_eq!(table.get_linecol(8), LineCol { line: 1, col: 3 });
        assert_eq!(table.get_linecol(10), LineCol { line: 2, col: 0 });
        assert_eq!(table.get_linecol(16), LineCol { line: 3, col: 5 });
    }

    #[test]
    fn line_count() {
        let table = create_line_table();
        assert_eq!(table.line_count(), 4);
    }

    #[test]
    fn line_start() {
        let table = create_line_table();
        assert_eq!(table.get_line_start(0), 0);
        assert_eq!(table.get_line_start(1), 5);
    }

    #[test]
    #[should_panic]
    fn line_start_past_end() {
        let table = create_line_table();
        table.get_line_start(4);
    }
}
