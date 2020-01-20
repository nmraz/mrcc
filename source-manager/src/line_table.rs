use std::vec::Vec;

use crate::source_pos::LineCol;

pub struct LineTable {
    line_offsets: Vec<u32>,
}

impl LineTable {
    pub fn new_for_src(src: &str) -> Self {
        let mut line_offsets = vec![0];

        for (c, off) in src.chars().zip(0..) {
            if c == '\n' {
                line_offsets.push(off + 1);
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
