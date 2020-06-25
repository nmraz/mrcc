use crate::SourcePos;

pub struct PendingIf {
    pub pos: SourcePos,
}

#[derive(Default)]
pub struct FileState {
    pub line_start: bool,
    pub pending_ifs: Vec<PendingIf>,
}
