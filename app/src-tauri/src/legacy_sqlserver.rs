//! SQL Server one-time import reader (v1.1 Task 9, spec §4.1). The
//! desktop shell connects with `tiberius`, reads each project table as
//! typed values and serialises them as EPTB v1 batches — the same
//! intermediate form `epiora-mdb` and the sqlite-wasm worker emit — so
//! Task 7's `map_ei7` maps a SQL Server project exactly once, as
//! `SourceFormat::Ei7SqlServer`.
//!
//! Credentials (spec §4.5, §15 review focus 7): [`SqlServerConn`]
//! arrives per command call and is held in memory for that call only —
//! never written to the store, settings or logs. [`SqlServerError`]
//! carries nothing but a code; a rejected login must not echo host,
//! user or password back to the UI.
//!
//! Type mapping (`fixtures/legacy/copy-tables.mjs` writes the same
//! mapping into a T-SQL copy): int-sized types and `bit` → the EPTB
//! int/bool kinds; float/money/decimal → real; `datetime` family → the
//! Access serial-days kind (T-SQL epochs converted to the OLE
//! 1899-12-30 epoch, see [`tds_serial`]); character, guid and xml →
//! text; binary/image/UDT/variant → binary-skipped, matching the mdb
//! reader's "not imported" columns. `verbatim` copies store datetimes
//! as `NVARCHAR` text, which the mapper parses on the SQLite-proven
//! path.

use futures_util::TryStreamExt;
use serde::Deserialize;
use tiberius::{AuthMethod, Client, Config};
use tokio::net::TcpStream;
use tokio_util::compat::{Compat, TokioAsyncReadCompatExt};

mod eptb;

use eptb::{col_acc, is_utf16_type, ChunkEncoder, ColAcc};

/// Connection parameters for one import call. Serialized over IPC per
/// call (`camelCase` on the JS side) and never persisted. No `Debug`/
/// `Display`: nothing may format the password into a log.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SqlServerConn {
    /// Server host name or IP.
    pub host: String,
    /// TCP port (1433 for the default instance).
    pub port: u16,
    /// Database containing the EI7 project tables.
    pub database: String,
    /// How to authenticate.
    pub auth: SqlServerAuth,
    /// Skip TLS certificate validation (self-signed container/dev
    /// servers; the channel stays encrypted).
    pub trust_cert: bool,
}

/// Authentication mode: SQL Server login, or Windows integrated auth
/// (desktop builds on Windows; elsewhere a typed `sqlserver_auth`
/// failure — there is no SSPI/GSSAPI outside them).
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SqlServerAuth {
    /// SQL Server login (`{ "sql": { "user", "password" } }`).
    Sql {
        /// Login name.
        user: String,
        /// Password — in memory for the call only.
        password: String,
    },
    /// Integrated Windows authentication (`"windows"`).
    Windows,
}

/// A SQL Server import failure. `Display` prints only [`Self::code`]
/// so the string that crosses IPC can never leak connection detail
/// (spec §10).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SqlServerError {
    code: &'static str,
}

impl SqlServerError {
    /// TCP/TLS/prelogin failure (or a non-login server error while
    /// connecting — e.g. an unknown database).
    pub const CONNECT: Self = Self {
        code: "sqlserver_connect",
    };
    /// Login rejected (SQL Server error 18456) or the requested auth
    /// mode is unavailable on this platform.
    pub const AUTH: Self = Self {
        code: "sqlserver_auth",
    };
    /// A table/schema/row read failed after login.
    pub const QUERY: Self = Self {
        code: "sqlserver_query",
    };

    /// The stable wire code.
    pub fn code(&self) -> &'static str {
        self.code
    }
}

impl std::fmt::Display for SqlServerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.code)
    }
}

impl std::error::Error for SqlServerError {}

type TClient = Client<Compat<TcpStream>>;

fn auth_method(auth: &SqlServerAuth) -> Result<AuthMethod, SqlServerError> {
    match auth {
        SqlServerAuth::Sql { user, password } => Ok(AuthMethod::sql_server(user, password)),
        SqlServerAuth::Windows => windows_auth(),
    }
}

#[cfg(windows)]
fn windows_auth() -> Result<AuthMethod, SqlServerError> {
    Ok(AuthMethod::Integrated)
}

#[cfg(not(windows))]
fn windows_auth() -> Result<AuthMethod, SqlServerError> {
    Err(SqlServerError::AUTH)
}

async fn connect(conn: &SqlServerConn) -> Result<TClient, SqlServerError> {
    let mut config = Config::new();
    config.host(&conn.host);
    config.port(conn.port);
    config.database(&conn.database);
    config.authentication(auth_method(&conn.auth)?);
    if conn.trust_cert {
        config.trust_cert();
    }
    let addr = config.get_addr();
    let tcp = TcpStream::connect(&addr)
        .await
        .map_err(|_| SqlServerError::CONNECT)?;
    let _ = tcp.set_nodelay(true);
    // The whole login handshake happens inside `Client::connect`, so a
    // `Server` error here is the server rejecting pre-login/login —
    // 18456 is "Login failed for user" specifically.
    match Client::connect(config, tcp.compat()).await {
        Ok(client) => Ok(client),
        Err(tiberius::error::Error::Server(e)) if e.code() == 18456 => Err(SqlServerError::AUTH),
        Err(_) => Err(SqlServerError::CONNECT),
    }
}

fn bracket(name: &str) -> String {
    format!("[{}]", name.replace(']', "]]"))
}

fn sql_lit(s: &str) -> String {
    format!("N'{}'", s.replace('\'', "''"))
}

/// The project's tables in creation order (`sys.tables.object_id` is
/// allocated at CREATE), mirroring `mdb-tables`/`sqlite3 .tables`.
/// `is_ms_shipped = 0` keeps only user tables.
pub async fn sqlserver_tables(conn: &SqlServerConn) -> Result<Vec<String>, SqlServerError> {
    let mut client = connect(conn).await?;
    let rows = client
        .simple_query("SELECT name FROM sys.tables WHERE is_ms_shipped = 0 ORDER BY object_id")
        .await
        .map_err(|_| SqlServerError::QUERY)?
        .into_first_result()
        .await
        .map_err(|_| SqlServerError::QUERY)?;
    let mut names = Vec::with_capacity(rows.len());
    for row in &rows {
        names.push(
            row.get::<&str, _>(0)
                .ok_or(SqlServerError::QUERY)?
                .to_string(),
        );
    }
    Ok(names)
}

/// One table as EPTB v1 chunks of at most `chunk_rows` rows each
/// (always at least one chunk, so an empty table still describes its
/// columns). `table` is a `sys.tables` name; it is bracket-quoted —
/// identifiers can't be bound parameters. Unicode text columns are
/// selected as `CAST(x AS VARBINARY(MAX))`: a verbatim `0x`→nvarchar
/// copy (or any byte pair that is not a valid UTF-16 sequence) would
/// otherwise abort tiberius' strict `String::from_utf16` decode
/// mid-stream; the bytes decode client-side with U+FFFD
/// substitution — the mdb reader's contract.
pub async fn sqlserver_batches(
    conn: &SqlServerConn,
    table: &str,
    chunk_rows: u32,
) -> Result<Vec<Vec<u8>>, SqlServerError> {
    let chunk_rows = chunk_rows.max(1) as usize;
    let mut client = connect(conn).await?;
    let meta = client
        .simple_query(format!(
            "SELECT c.name, c.system_type_id FROM sys.columns c \
             WHERE c.object_id = OBJECT_ID({}, N'U') ORDER BY c.column_id",
            sql_lit(table)
        ))
        .await
        .map_err(|_| SqlServerError::QUERY)?
        .into_first_result()
        .await
        .map_err(|_| SqlServerError::QUERY)?;
    let mut exprs = Vec::with_capacity(meta.len());
    let mut accs: Vec<ColAcc> = Vec::with_capacity(meta.len());
    for row in &meta {
        let name = row
            .get::<&str, _>(0)
            .ok_or(SqlServerError::QUERY)?
            .to_string();
        let type_id = row.get::<u8, _>(1).ok_or(SqlServerError::QUERY)?;
        let q = bracket(&name);
        exprs.push(if is_utf16_type(type_id) {
            format!("CAST({q} AS VARBINARY(MAX)) AS {q}")
        } else {
            q
        });
        accs.push(col_acc(&name, type_id));
    }
    // Row order must match the file order the Access golden was built
    // from — `metaViews`, `metaLinks` and the `code*` lookup tables
    // all feed order-sensitive output — while a heap `SELECT` has no
    // guaranteed order at all. EI7's data/meta tables lead with the
    // row key (`UniqueKey`/`XxxId`, int family): ordering by it is
    // the key order both for the verbatim copy and for a real EI7
    // project. Tables whose first column is not an integer (the
    // `code*` lookups — text first column) get `%%physloc%%`: the
    // physical slot order, i.e. insert order for a freshly loaded
    // heap and key order on a clustered table.
    let first = meta.first().ok_or(SqlServerError::QUERY)?;
    let first_name = first.get::<&str, _>(0).ok_or(SqlServerError::QUERY)?;
    let first_type = first.get::<u8, _>(1).ok_or(SqlServerError::QUERY)?;
    let order = if eptb::is_int_type(first_type) {
        format!(" ORDER BY {}", bracket(first_name))
    } else {
        " ORDER BY %%physloc%%".to_string()
    };
    let sql = format!(
        "SELECT {} FROM {}{}",
        exprs.join(", "),
        bracket(table),
        order
    );
    let mut stream = client
        .query(sql, &[])
        .await
        .map_err(|_| SqlServerError::QUERY)?;
    let mut encoder = ChunkEncoder::new(table, accs);
    while let Some(item) = stream.try_next().await.map_err(|_| SqlServerError::QUERY)? {
        let Some(row) = item.as_row() else { continue };
        encoder.push_row(row);
        if encoder.row_count == chunk_rows {
            encoder.flush();
        }
    }
    encoder.flush();
    Ok(encoder.chunks)
}
