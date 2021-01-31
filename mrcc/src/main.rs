#![warn(rust_2018_idioms)]

use std::fs;
use std::path::PathBuf;

use structopt::StructOpt;

use lex::{Interner, LexCtx, TokenKind};
use pp::PreprocessorBuilder;
use source::smap::{FileContents, FileName, SourceMap};
use source::{diag::Level, DResult, DiagManager};

#[derive(StructOpt)]
struct Opts {
    pub filename: PathBuf,
}

fn run(diags: &mut DiagManager<'_>) -> DResult<()> {
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
        if ppt.data() == TokenKind::Eof {
            break;
        }

        if ppt.line_start {
            println!();

            // Preserve indentation by advancing to the start column first.
            let col = ctx
                .smap
                .get_interpreted_range(ctx.smap.get_replacement_range(ppt.range()))
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
    let mut diags = DiagManager::new_annotating(None);

    if run(&mut diags).is_err() || diags.error_count() > 0 {
        std::process::exit(1);
    }
}
