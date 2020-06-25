use crate::SourcePos;

pub struct PendingIf {
    pub pos: SourcePos,
}

pub struct FileState {
    pub line_start: bool,
    pub pending_ifs: Vec<PendingIf>,
}

impl Default for FileState {
    fn default() -> Self {
        Self {
            line_start: true,
            pending_ifs: vec![],
        }
    }
}
