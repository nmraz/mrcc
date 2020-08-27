use std::fs;
use std::iter;
use std::path::PathBuf;

use structopt::StructOpt;

use frontend::lex::{Interner, LexCtx, TokenKind};
use frontend::pp::PreprocessorBuilder;
use frontend::smap::{FileContents, FileName, SourceMap};
use frontend::{
    diag::{Level, Ranges, RenderedDiagnostic, RenderedHandler, RenderedSubDiagnostic},
    DResult, DiagManager,
};

#[derive(StructOpt)]
struct Opts {
    pub filename: PathBuf,
}

struct Handler;

impl RenderedHandler for Handler {
    fn handle(&mut self, diag: &RenderedDiagnostic<'_>) {
        let subdiags =
            iter::once((diag.level, &diag.main)).chain(iter::repeat(Level::Note).zip(&diag.notes));

        match diag.smap {
            Some(smap) => subdiags.for_each(|(level, subdiag)| print_subdiag(level, subdiag, smap)),
            None => subdiags.for_each(|(level, subdiag)| print_anon_subdiag(level, subdiag)),
        }
    }
}

fn print_subdiag(level: Level, subdiag: &RenderedSubDiagnostic, smap: &SourceMap) {
    match subdiag.ranges() {
        Some(&Ranges { primary_range, .. }) => {
            let interpreted = smap.get_interpreted_range(primary_range);
            let linecol = interpreted.start_linecol();
            eprintln!(
                "{}:{}:{}: {}: {}",
                interpreted.filename(),
                linecol.line + 1,
                linecol.col + 1,
                level,
                subdiag.msg()
            )
        }
        None => print_anon_subdiag(level, subdiag),
    }
}

fn print_anon_subdiag(level: Level, subdiag: &RenderedSubDiagnostic) {
    eprintln!("{}: {}", level, subdiag.msg())
}

fn run(diags: &mut DiagManager) -> DResult<()> {
    let opts = Opts::from_args();

    let main_src = fs::read_to_string(&opts.filename).map_err(|err| {
        diags
            .report_anon(
                Level::Fatal,
                format!("failed to read '{}': {}", opts.filename.display(), err),
            )
            .emit()
            .unwrap_err()
    })?;

    let mut interner = Interner::new();
    let mut smap = SourceMap::new();

    let main_id = smap
        .create_file(
            FileName::real(opts.filename.clone()),
            FileContents::new(&main_src),
            None,
        )
        .map_err(|_| {
            diags
                .report_anon(Level::Fatal, "file too large".into())
                .emit()
                .unwrap_err()
        })?;

    let mut ctx = LexCtx::new(&mut interner, diags, &mut smap);

    let mut pp = PreprocessorBuilder::new(&mut ctx, main_id)
        .parent_dir(opts.filename.parent().unwrap().into())
        .build();

    loop {
        let ppt = pp.next_pp(&mut ctx)?;
        if ppt.kind() == TokenKind::Eof {
            break;
        }

        if ppt.line_start {
            println!();

            // Preserve indentation by advancing to the start column first.
            let col = ctx
                .smap
                .get_interpreted_range(ctx.smap.get_expansion_range(ppt.range()))
                .start_linecol()
                .col;

            print!("{}", " ".repeat(col as usize));

            // We've already handled the leading whitespace ourselves, output the token directly.
            print!("{}", ppt.tok.display(&ctx))
        } else {
            print!("{}", ppt.display(&ctx));
        }
    }

    Ok(())
}

fn main() {
    let mut diags = DiagManager::with_rendered_handler(Handler, None);

    if run(&mut diags).is_err() || diags.error_count() > 0 {
        std::process::exit(1);
    }
}
