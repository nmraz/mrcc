use mrcc_source::SourcePos;

use super::processor::FileToken;

pub struct PendingIf {
    pub pos: SourcePos,
}

pub struct FileState {
    pub line_start: bool,
    pub lookahead: Option<FileToken>,
    pub pending_ifs: Vec<PendingIf>,
}

impl Default for FileState {
    fn default() -> Self {
        Self {
            line_start: true,
            lookahead: None,
            pending_ifs: vec![],
        }
    }
}
