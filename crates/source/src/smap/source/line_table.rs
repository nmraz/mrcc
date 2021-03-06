use std::convert::TryFrom;
use std::vec::Vec;

use crate::{LineCol, LocalOff};

pub struct LineTable {
    /// Holds the starting offsets of lines in the source
    line_offsets: Vec<LocalOff>,
}

impl LineTable {
    pub fn new_for_src(src: &str) -> Self {
        let mut line_offsets = vec![0.into()];

        for (off, &c) in src.as_bytes().iter().enumerate() {
            if c == b'\n' {
                line_offsets.push(LocalOff::try_from(off + 1).unwrap());
            }
        }

        LineTable { line_offsets }
    }

    pub fn get_linecol(&self, off: LocalOff) -> LineCol {
        let line = self
            .line_offsets
            .binary_search(&off)
            .unwrap_or_else(|i| i - 1);

        let col = (off - self.line_offsets[line]).into();

        LineCol {
            line: line as u32,
            col,
        }
    }

    pub fn line_count(&self) -> u32 {
        self.line_offsets.len() as u32
    }

    pub fn get_line_start(&self, line: u32) -> LocalOff {
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
        assert_eq!(table.get_linecol(0.into()), LineCol { line: 0, col: 0 });
        assert_eq!(table.get_linecol(2.into()), LineCol { line: 0, col: 2 });
        assert_eq!(table.get_linecol(8.into()), LineCol { line: 1, col: 3 });
        assert_eq!(table.get_linecol(10.into()), LineCol { line: 2, col: 0 });
        assert_eq!(table.get_linecol(16.into()), LineCol { line: 3, col: 5 });
    }

    #[test]
    fn line_count() {
        let table = create_line_table();
        assert_eq!(table.line_count(), 4);
    }

    #[test]
    fn line_start() {
        let table = create_line_table();
        assert_eq!(table.get_line_start(0), 0.into());
        assert_eq!(table.get_line_start(1), 5.into());
    }

    #[test]
    #[should_panic]
    fn line_start_past_end() {
        let table = create_line_table();
        table.get_line_start(4);
    }
}
