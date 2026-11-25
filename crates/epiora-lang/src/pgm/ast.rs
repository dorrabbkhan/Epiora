//! The `.pgm` AST (v1.1 spec §6.1): every parsed statement keeps its
//! 1-based line and column, whether or not Epiora runs it yet —
//! "parsing never depends on support".

use crate::manage::Command;
use crate::view::DatasetRef;

/// A parsed program: one spanned statement per command line (block
/// commands — IF, RECODE — are a single statement).
#[derive(Clone, Debug, PartialEq)]
pub struct Program {
    /// Statements in source order.
    pub statements: Vec<Spanned<Statement>>,
}

/// A node with its 1-based source position.
#[derive(Clone, Debug, PartialEq)]
pub struct Spanned<T> {
    /// 1-based line.
    pub line: u32,
    /// 1-based column.
    pub column: u32,
    /// The node.
    pub node: T,
}

/// One `.pgm` statement. Commands Epiora runs keep their structured
/// form; the rest keep their name and source text so the migration
/// report can list them.
#[derive(Clone, Debug, PartialEq)]
pub enum Statement {
    /// A cleaning command (spec §5.3's `Command`).
    Manage(Command),
    /// FREQ / TABLES / MEANS — a procedure name plus its options as
    /// parsed JSON (`{"vars": [...], "opts": {...}}`; `*` →
    /// `"all": true`, `EXCEPT` → `"except": [...]`).
    Procedure {
        /// `FREQ` / `TABLES` / `MEANS`.
        name: String,
        /// Parsed variables and option words.
        options: serde_json::Value,
    },
    /// `READ` — a dataset or `file:table` target.
    Read(DatasetRef),
    /// `WRITE` — materialize the working view.
    Write(WriteTarget),
    /// `LIST` — variable names as spelled (`["*"]` for `LIST *`,
    /// `[]` for a bare `LIST`).
    List(Vec<String>),
    /// `DISPLAY` — which system view to show.
    Display(DisplayWhat),
    /// An output-only command (`TYPEOUT`, `HEADER`, `ROUTEOUT`,
    /// `CLOSEOUT`, `NEWPAGE`, `PRINTOUT`, `SYSINFO`, `ABOUT`,
    /// `BEEP`, display-only `SET` clauses): its source text, kept as
    /// a note.
    Output(String),
    /// `RUNPGM` — the program to run.
    RunPgm(ProgramRef),
    /// `QUIT` or `EXIT` — ends the plan at its line.
    Quit,
    /// A recognized command Epiora does not run yet (or a supported
    /// command in a form outside the implemented subset): the
    /// command's register name and its full source text.
    Unsupported {
        /// The longest register name the statement starts with, else
        /// its head keyword (upper-cased).
        name: String,
        /// The statement's source text, trimmed.
        text: String,
    },
}

/// `WRITE`'s destination and record selection.
#[derive(Clone, Debug, PartialEq)]
pub struct WriteTarget {
    /// `REPLACE` or `APPEND`.
    pub mode: WriteMode,
    /// The output format string as spelled (`"Epi 2000"`, …).
    pub format: String,
    /// `table` or `file:table`.
    pub target: String,
    /// Which variables to write.
    pub vars: WriteVars,
}

/// `WRITE`'s mode word.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WriteMode {
    /// Overwrite the target.
    Replace,
    /// Append to the target.
    Append,
}

/// `WRITE`'s variable selection.
#[derive(Clone, Debug, PartialEq)]
pub enum WriteVars {
    /// `*` — every variable.
    All,
    /// `* EXCEPT v…`.
    Except(Vec<String>),
    /// An explicit variable list.
    Only(Vec<String>),
}

/// `DISPLAY`'s subject.
#[derive(Clone, Debug, PartialEq)]
pub enum DisplayWhat {
    /// `DISPLAY DBVARIABLES [vars]`.
    DbVariables {
        /// The listed variables (empty = all).
        fields: Vec<String>,
    },
    /// `DISPLAY DBVIEWS`.
    DbViews,
    /// `DISPLAY TABLES`.
    Tables,
    /// `DISPLAY RECIPE` — the session's recipe text (register
    /// extension; not an EI7 grammar word).
    Recipe,
}

/// `RUNPGM`'s target (the register's `program in project` / `File` /
/// `File:program` forms).
#[derive(Clone, Debug, PartialEq)]
pub enum ProgramRef {
    /// A program saved in the project: `RUNPGM name` or
    /// `RUNPGM "name"`.
    Named(String),
    /// A `.pgm` file: `RUNPGM 'file'` or a bare path.
    File(String),
    /// A program inside a file or project: `RUNPGM 'file':name` or
    /// `RUNPGM "file":name`.
    FileProgram {
        /// The file or project path.
        file: String,
        /// The program name inside it.
        name: String,
    },
}
