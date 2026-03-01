use std::path::Path;

use rusqlite::{params, Connection, Transaction};
use tracing::{debug, error, info, trace};

use crate::error::CctError;
use crate::models::graph::ParseStatistics;
use crate::models::relation::{
    CallRelation, FileInfo, FileParseStatus, IncludeRelation, InheritanceRelation, RefKind,
    ReferenceRelation,
};
use crate::models::symbol::{Access, Symbol, SymbolKind};

/// 索引数据库 — 基于 SQLite 的持久化存储层
///
/// # 设计说明（外观模式）
/// 封装 rusqlite 底层操作，为上层提供以业务语义命名的接口，
/// 隐藏 SQL 细节和事务管理逻辑。
pub struct IndexDatabase {
    conn: Connection,
}

impl IndexDatabase {
    /// 打开或创建索引数据库
    ///
    /// 自动启用 WAL 模式以提升并发读写性能。
    ///
    /// # 参数
    /// - `db_path`: 数据库文件路径；若文件不存在则自动创建
    pub fn open(db_path: &Path) -> Result<Self, CctError> {
        info!(path = %db_path.display(), "IndexDatabase::open 打开索引数据库");

        let conn = Connection::open(db_path).map_err(|e| {
            error!(path = %db_path.display(), error = %e, "无法打开数据库");
            CctError::Database(format!("打开数据库失败: {e}"))
        })?;

        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")
            .map_err(|e| {
                error!(error = %e, "设置数据库 PRAGMA 失败");
                CctError::Database(format!("PRAGMA 设置失败: {e}"))
            })?;

        debug!("数据库已打开，WAL 模式已启用");
        Ok(Self { conn })
    }

    /// 初始化数据库表结构 — 对应 doc/02 §4.2 DDL
    ///
    /// 使用 `IF NOT EXISTS` 保证幂等性，可安全重复调用。
    pub fn initialize(&self) -> Result<(), CctError> {
        info!("IndexDatabase::initialize 初始化数据库表结构");

        self.conn
            .execute_batch(
                "
            CREATE TABLE IF NOT EXISTS symbols (
                id              INTEGER PRIMARY KEY,
                name            TEXT NOT NULL,
                qualified_name  TEXT NOT NULL,
                kind            TEXT NOT NULL,
                sub_kind        TEXT,
                file_path       TEXT NOT NULL,
                line            INTEGER NOT NULL,
                column          INTEGER NOT NULL,
                end_line        INTEGER,
                is_definition   BOOLEAN NOT NULL DEFAULT 0,
                return_type     TEXT,
                parameters      TEXT,
                access          TEXT,
                attributes      TEXT,
                project_id      TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_symbols_name      ON symbols(name);
            CREATE INDEX IF NOT EXISTS idx_symbols_qualified  ON symbols(qualified_name);
            CREATE INDEX IF NOT EXISTS idx_symbols_file       ON symbols(file_path);
            CREATE INDEX IF NOT EXISTS idx_symbols_kind       ON symbols(kind);

            CREATE TABLE IF NOT EXISTS call_relations (
                id          INTEGER PRIMARY KEY,
                caller_id   INTEGER NOT NULL REFERENCES symbols(id),
                callee_id   INTEGER NOT NULL REFERENCES symbols(id),
                file_path   TEXT NOT NULL,
                line        INTEGER NOT NULL,
                column      INTEGER NOT NULL,
                is_virtual  BOOLEAN NOT NULL DEFAULT 0,
                is_indirect BOOLEAN NOT NULL DEFAULT 0
            );

            CREATE INDEX IF NOT EXISTS idx_calls_caller ON call_relations(caller_id);
            CREATE INDEX IF NOT EXISTS idx_calls_callee ON call_relations(callee_id);

            CREATE TABLE IF NOT EXISTS include_relations (
                id            INTEGER PRIMARY KEY,
                source_file   TEXT NOT NULL,
                target_file   TEXT NOT NULL,
                line          INTEGER NOT NULL,
                is_system     BOOLEAN NOT NULL DEFAULT 0,
                resolved_path TEXT
            );

            CREATE INDEX IF NOT EXISTS idx_includes_source ON include_relations(source_file);
            CREATE INDEX IF NOT EXISTS idx_includes_target ON include_relations(target_file);

            CREATE TABLE IF NOT EXISTS reference_relations (
                id          INTEGER PRIMARY KEY,
                symbol_id   INTEGER NOT NULL REFERENCES symbols(id),
                file_path   TEXT NOT NULL,
                line        INTEGER NOT NULL,
                column      INTEGER NOT NULL,
                ref_kind    TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_refs_symbol ON reference_relations(symbol_id);
            CREATE INDEX IF NOT EXISTS idx_refs_file   ON reference_relations(file_path);

            CREATE TABLE IF NOT EXISTS inheritance_relations (
                id          INTEGER PRIMARY KEY,
                derived_id  INTEGER NOT NULL REFERENCES symbols(id),
                base_id     INTEGER NOT NULL REFERENCES symbols(id),
                access      TEXT NOT NULL,
                is_virtual  BOOLEAN NOT NULL DEFAULT 0
            );

            CREATE TABLE IF NOT EXISTS file_info (
                file_path     TEXT PRIMARY KEY,
                last_modified INTEGER NOT NULL,
                content_hash  TEXT NOT NULL,
                parse_status  TEXT NOT NULL,
                error_message TEXT,
                symbol_count  INTEGER NOT NULL DEFAULT 0,
                parse_time_ms INTEGER
            );
            ",
            )
            .map_err(|e| {
                error!(error = %e, "创建数据库表失败");
                CctError::Database(format!("表结构初始化失败: {e}"))
            })?;

        debug!("数据库表结构初始化完成");
        Ok(())
    }

    /// 批量插入符号记录
    ///
    /// 使用事务保证原子性，一次性写入所有符号。
    pub fn insert_symbols(&mut self, symbols: &[Symbol]) -> Result<(), CctError> {
        info!(count = symbols.len(), "IndexDatabase::insert_symbols 批量插入符号");

        let tx = self.conn.transaction().map_err(map_db_err)?;
        {
            let mut stmt = tx
                .prepare_cached(
                    "INSERT INTO symbols (
                        id, name, qualified_name, kind, sub_kind, file_path,
                        line, column, end_line, is_definition, return_type,
                        parameters, access, attributes, project_id
                    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
                )
                .map_err(map_db_err)?;

            for s in symbols {
                trace!(id = s.id, name = %s.name, "插入符号");
                stmt.execute(params![
                    s.id,
                    s.name,
                    s.qualified_name,
                    symbol_kind_to_str(&s.kind),
                    s.sub_kind,
                    s.file_path,
                    s.line,
                    s.column,
                    s.end_line,
                    s.is_definition,
                    s.return_type,
                    s.parameters,
                    s.access.as_ref().map(access_to_str),
                    s.attributes,
                    s.project_id,
                ])
                .map_err(map_db_err)?;
            }
        }
        tx.commit().map_err(map_db_err)?;

        debug!(count = symbols.len(), "符号插入完成");
        Ok(())
    }

    /// 批量插入调用关系
    pub fn insert_call_relations(&mut self, relations: &[CallRelation]) -> Result<(), CctError> {
        info!(
            count = relations.len(),
            "IndexDatabase::insert_call_relations 批量插入调用关系"
        );

        let tx = self.conn.transaction().map_err(map_db_err)?;
        batch_insert_call_relations(&tx, relations)?;
        tx.commit().map_err(map_db_err)?;

        debug!(count = relations.len(), "调用关系插入完成");
        Ok(())
    }

    /// 批量插入包含关系
    pub fn insert_include_relations(
        &mut self,
        relations: &[IncludeRelation],
    ) -> Result<(), CctError> {
        info!(
            count = relations.len(),
            "IndexDatabase::insert_include_relations 批量插入包含关系"
        );

        let tx = self.conn.transaction().map_err(map_db_err)?;
        batch_insert_include_relations(&tx, relations)?;
        tx.commit().map_err(map_db_err)?;

        debug!(count = relations.len(), "包含关系插入完成");
        Ok(())
    }

    /// 批量插入引用关系
    pub fn insert_reference_relations(
        &mut self,
        relations: &[ReferenceRelation],
    ) -> Result<(), CctError> {
        info!(
            count = relations.len(),
            "IndexDatabase::insert_reference_relations 批量插入引用关系"
        );

        let tx = self.conn.transaction().map_err(map_db_err)?;
        batch_insert_reference_relations(&tx, relations)?;
        tx.commit().map_err(map_db_err)?;

        debug!(count = relations.len(), "引用关系插入完成");
        Ok(())
    }

    /// 批量插入继承关系
    pub fn insert_inheritance_relations(
        &mut self,
        relations: &[InheritanceRelation],
    ) -> Result<(), CctError> {
        info!(
            count = relations.len(),
            "IndexDatabase::insert_inheritance_relations 批量插入继承关系"
        );

        let tx = self.conn.transaction().map_err(map_db_err)?;
        batch_insert_inheritance_relations(&tx, relations)?;
        tx.commit().map_err(map_db_err)?;

        debug!(count = relations.len(), "继承关系插入完成");
        Ok(())
    }

    /// 插入或更新文件信息（增量解析用）
    ///
    /// 若文件记录已存在则更新，否则插入新记录。
    pub fn upsert_file_info(&self, info: &FileInfo) -> Result<(), CctError> {
        debug!(
            file = %info.file_path,
            status = ?info.parse_status,
            "IndexDatabase::upsert_file_info 更新文件信息"
        );

        self.conn
            .execute(
                "INSERT INTO file_info (
                    file_path, last_modified, content_hash, parse_status,
                    error_message, symbol_count, parse_time_ms
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                ON CONFLICT(file_path) DO UPDATE SET
                    last_modified = excluded.last_modified,
                    content_hash  = excluded.content_hash,
                    parse_status  = excluded.parse_status,
                    error_message = excluded.error_message,
                    symbol_count  = excluded.symbol_count,
                    parse_time_ms = excluded.parse_time_ms",
                params![
                    info.file_path,
                    info.last_modified,
                    info.content_hash,
                    parse_status_to_str(&info.parse_status),
                    info.error_message,
                    info.symbol_count,
                    info.parse_time_ms,
                ],
            )
            .map_err(map_db_err)?;

        trace!(file = %info.file_path, "文件信息已更新");
        Ok(())
    }

    /// 查询文件信息
    ///
    /// # 参数
    /// - `file_path`: 文件路径（与写入时一致的字符串）
    ///
    /// # 返回
    /// 存在时返回 `Some(FileInfo)`，不存在时返回 `None`。
    pub fn get_file_info(&self, file_path: &str) -> Result<Option<FileInfo>, CctError> {
        debug!(file = file_path, "IndexDatabase::get_file_info 查询文件信息");

        let mut stmt = self
            .conn
            .prepare_cached(
                "SELECT file_path, last_modified, content_hash, parse_status,
                        error_message, symbol_count, parse_time_ms
                 FROM file_info WHERE file_path = ?1",
            )
            .map_err(map_db_err)?;

        let result = stmt
            .query_row(params![file_path], |row| {
                Ok(FileInfo {
                    file_path: row.get(0)?,
                    last_modified: row.get(1)?,
                    content_hash: row.get(2)?,
                    parse_status: str_to_parse_status(&row.get::<_, String>(3)?),
                    error_message: row.get(4)?,
                    symbol_count: row.get(5)?,
                    parse_time_ms: row.get(6)?,
                })
            })
            .optional()
            .map_err(map_db_err)?;

        trace!(file = file_path, found = result.is_some(), "查询完成");
        Ok(result)
    }

    /// 清除指定文件的所有索引数据
    ///
    /// 删除该文件关联的符号、关系和文件信息记录，用于增量解析前的清理。
    pub fn clear_file_data(&mut self, file_path: &str) -> Result<(), CctError> {
        info!(file = file_path, "IndexDatabase::clear_file_data 清除文件索引数据");

        let tx = self.conn.transaction().map_err(map_db_err)?;

        tx.execute(
            "DELETE FROM call_relations WHERE file_path = ?1",
            params![file_path],
        )
        .map_err(map_db_err)?;

        tx.execute(
            "DELETE FROM reference_relations WHERE file_path = ?1",
            params![file_path],
        )
        .map_err(map_db_err)?;

        tx.execute(
            "DELETE FROM include_relations WHERE source_file = ?1",
            params![file_path],
        )
        .map_err(map_db_err)?;

        tx.execute(
            "DELETE FROM inheritance_relations WHERE derived_id IN \
             (SELECT id FROM symbols WHERE file_path = ?1)",
            params![file_path],
        )
        .map_err(map_db_err)?;

        tx.execute(
            "DELETE FROM symbols WHERE file_path = ?1",
            params![file_path],
        )
        .map_err(map_db_err)?;

        tx.execute(
            "DELETE FROM file_info WHERE file_path = ?1",
            params![file_path],
        )
        .map_err(map_db_err)?;

        tx.commit().map_err(map_db_err)?;

        debug!(file = file_path, "文件索引数据已清除");
        Ok(())
    }

    /// 获取当前索引的统计信息
    pub fn get_statistics(&self) -> Result<ParseStatistics, CctError> {
        info!("IndexDatabase::get_statistics 查询索引统计信息");

        let count = |table: &str| -> Result<u64, CctError> {
            let sql = format!("SELECT COUNT(*) FROM {table}");
            self.conn
                .query_row(&sql, [], |row| row.get::<_, u64>(0))
                .map_err(map_db_err)
        };

        let kind_count = |kind: &str| -> Result<u64, CctError> {
            self.conn
                .query_row(
                    "SELECT COUNT(*) FROM symbols WHERE kind = ?1",
                    params![kind],
                    |row| row.get::<_, u64>(0),
                )
                .map_err(map_db_err)
        };

        let file_count = |status: &str| -> Result<u64, CctError> {
            self.conn
                .query_row(
                    "SELECT COUNT(*) FROM file_info WHERE parse_status = ?1",
                    params![status],
                    |row| row.get::<_, u64>(0),
                )
                .map_err(map_db_err)
        };

        let total_files = count("file_info")?;
        let parsed_files = file_count("success")?;
        let failed_files = file_count("failed")?;

        let stats = ParseStatistics {
            total_files,
            parsed_files,
            failed_files,
            total_symbols: count("symbols")?,
            total_functions: kind_count("function")?,
            total_variables: kind_count("variable")?,
            total_types: kind_count("type")?,
            total_macros: kind_count("macro")?,
            total_call_relations: count("call_relations")?,
            total_include_relations: count("include_relations")?,
            total_reference_relations: count("reference_relations")?,
            total_inheritance_relations: count("inheritance_relations")?,
            elapsed_seconds: 0.0,
        };

        debug!(?stats, "统计信息查询完成");
        Ok(stats)
    }
}

// ── 私有辅助函数 ──────────────────────────────────────────────────────

fn map_db_err(e: rusqlite::Error) -> CctError {
    CctError::Database(e.to_string())
}

pub fn symbol_kind_to_str(kind: &SymbolKind) -> &'static str {
    match kind {
        SymbolKind::Function => "function",
        SymbolKind::Variable => "variable",
        SymbolKind::Type => "type",
        SymbolKind::Macro => "macro",
    }
}

fn access_to_str(access: &Access) -> &'static str {
    match access {
        Access::Public => "public",
        Access::Protected => "protected",
        Access::Private => "private",
    }
}

fn parse_status_to_str(status: &FileParseStatus) -> &'static str {
    match status {
        FileParseStatus::Success => "success",
        FileParseStatus::Failed => "failed",
        FileParseStatus::Skipped => "skipped",
    }
}

fn str_to_parse_status(s: &str) -> FileParseStatus {
    match s {
        "success" => FileParseStatus::Success,
        "failed" => FileParseStatus::Failed,
        _ => FileParseStatus::Skipped,
    }
}

fn batch_insert_call_relations(
    tx: &Transaction<'_>,
    relations: &[CallRelation],
) -> Result<(), CctError> {
    let mut stmt = tx
        .prepare_cached(
            "INSERT INTO call_relations (
                id, caller_id, callee_id, file_path, line, column, is_virtual, is_indirect
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        )
        .map_err(map_db_err)?;

    for r in relations {
        trace!(id = r.id, caller = r.caller_id, callee = r.callee_id, "插入调用关系");
        stmt.execute(params![
            r.id,
            r.caller_id,
            r.callee_id,
            r.call_site_file,
            r.call_site_line,
            r.call_site_column,
            r.is_virtual_dispatch,
            r.is_indirect,
        ])
        .map_err(map_db_err)?;
    }
    Ok(())
}

fn batch_insert_include_relations(
    tx: &Transaction<'_>,
    relations: &[IncludeRelation],
) -> Result<(), CctError> {
    let mut stmt = tx
        .prepare_cached(
            "INSERT INTO include_relations (
                id, source_file, target_file, line, is_system, resolved_path
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        )
        .map_err(map_db_err)?;

    for r in relations {
        trace!(id = r.id, source = %r.source_file, target = %r.target_file, "插入包含关系");
        stmt.execute(params![
            r.id,
            r.source_file,
            r.target_file,
            r.include_line,
            r.is_system_header,
            r.resolved_path,
        ])
        .map_err(map_db_err)?;
    }
    Ok(())
}

fn batch_insert_reference_relations(
    tx: &Transaction<'_>,
    relations: &[ReferenceRelation],
) -> Result<(), CctError> {
    let mut stmt = tx
        .prepare_cached(
            "INSERT INTO reference_relations (
                id, symbol_id, file_path, line, column, ref_kind
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        )
        .map_err(map_db_err)?;

    for r in relations {
        trace!(id = r.id, symbol = r.symbol_id, kind = %r.reference_kind, "插入引用关系");
        stmt.execute(params![
            r.id,
            r.symbol_id,
            r.reference_file,
            r.reference_line,
            r.reference_column,
            ref_kind_to_str(&r.reference_kind),
        ])
        .map_err(map_db_err)?;
    }
    Ok(())
}

fn batch_insert_inheritance_relations(
    tx: &Transaction<'_>,
    relations: &[InheritanceRelation],
) -> Result<(), CctError> {
    let mut stmt = tx
        .prepare_cached(
            "INSERT INTO inheritance_relations (
                id, derived_id, base_id, access, is_virtual
            ) VALUES (?1, ?2, ?3, ?4, ?5)",
        )
        .map_err(map_db_err)?;

    for r in relations {
        trace!(
            id = r.id,
            derived = r.derived_class_id,
            base = r.base_class_id,
            "插入继承关系"
        );
        stmt.execute(params![
            r.id,
            r.derived_class_id,
            r.base_class_id,
            access_to_str(&r.access),
            r.is_virtual,
        ])
        .map_err(map_db_err)?;
    }
    Ok(())
}

fn ref_kind_to_str(kind: &RefKind) -> &'static str {
    match kind {
        RefKind::Read => "read",
        RefKind::Write => "write",
        RefKind::Address => "address",
        RefKind::Call => "call",
        RefKind::Type => "type",
    }
}

impl IndexDatabase {
    /// 暴露只读连接引用，供查询模块执行查询
    pub fn conn(&self) -> &Connection {
        &self.conn
    }

    /// 查询符号名称
    pub fn lookup_symbol_name(&self, id: i64) -> Option<String> {
        self.conn
            .query_row(
                "SELECT qualified_name FROM symbols WHERE id = ?1",
                params![id],
                |row| row.get(0),
            )
            .ok()
    }

    /// 查询符号类型
    pub fn lookup_symbol_kind(&self, id: i64) -> Option<SymbolKind> {
        self.conn
            .query_row(
                "SELECT kind FROM symbols WHERE id = ?1",
                params![id],
                |row| row.get::<_, String>(0),
            )
            .ok()
            .map(|s| str_to_symbol_kind(&s))
    }

    /// 查询符号所在文件
    pub fn lookup_symbol_file(&self, id: i64) -> Option<String> {
        self.conn
            .query_row(
                "SELECT file_path FROM symbols WHERE id = ?1",
                params![id],
                |row| row.get(0),
            )
            .ok()
    }

    /// 查询符号所在行
    pub fn lookup_symbol_line(&self, id: i64) -> Option<u32> {
        self.conn
            .query_row(
                "SELECT line FROM symbols WHERE id = ?1",
                params![id],
                |row| row.get(0),
            )
            .ok()
    }

    /// 按 ID 查询完整符号记录
    ///
    /// # 参数
    /// - `id`: 符号 ID
    ///
    /// # 返回
    /// 存在时返回 `Some(Symbol)`，不存在时返回 `None`
    pub fn lookup_symbol(&self, id: i64) -> Option<Symbol> {
        debug!(id = id, "IndexDatabase::lookup_symbol 按 ID 查询符号");
        self.conn
            .query_row(
                "SELECT id, name, qualified_name, kind, sub_kind, file_path, \
                 line, column, end_line, is_definition, return_type, \
                 parameters, access, attributes, project_id \
                 FROM symbols WHERE id = ?1",
                params![id],
                |row| row_to_symbol(row),
            )
            .ok()
    }
}

// ── 公共辅助函数 — 供 query 模块复用 ──────────────────────────────────

pub fn str_to_symbol_kind(s: &str) -> SymbolKind {
    match s {
        "function" => SymbolKind::Function,
        "variable" => SymbolKind::Variable,
        "type" => SymbolKind::Type,
        "macro" => SymbolKind::Macro,
        _ => SymbolKind::Function,
    }
}

pub fn str_to_access(s: &str) -> Access {
    match s {
        "public" => Access::Public,
        "protected" => Access::Protected,
        "private" => Access::Private,
        _ => Access::Public,
    }
}

pub fn str_to_ref_kind(s: &str) -> RefKind {
    match s {
        "read" => RefKind::Read,
        "write" => RefKind::Write,
        "address" => RefKind::Address,
        "call" => RefKind::Call,
        "type" => RefKind::Type,
        _ => RefKind::Read,
    }
}

/// 从数据库行映射为 Symbol 结构体（列顺序需与 SELECT 一致）
pub fn row_to_symbol(row: &rusqlite::Row) -> Result<Symbol, rusqlite::Error> {
    Ok(Symbol {
        id: row.get(0)?,
        name: row.get(1)?,
        qualified_name: row.get(2)?,
        kind: str_to_symbol_kind(&row.get::<_, String>(3)?),
        sub_kind: row.get(4)?,
        file_path: row.get(5)?,
        line: row.get(6)?,
        column: row.get(7)?,
        end_line: row.get(8)?,
        is_definition: row.get(9)?,
        return_type: row.get(10)?,
        parameters: row.get(11)?,
        access: row.get::<_, Option<String>>(12)?.map(|s| str_to_access(&s)),
        attributes: row.get(13)?,
        project_id: row.get(14)?,
    })
}

/// rusqlite `query_row` 辅助 — 将 QueryReturnedNoRows 映射为 None
trait OptionalExt<T> {
    fn optional(self) -> Result<Option<T>, rusqlite::Error>;
}

impl<T> OptionalExt<T> for Result<T, rusqlite::Error> {
    fn optional(self) -> Result<Option<T>, rusqlite::Error> {
        match self {
            Ok(v) => Ok(Some(v)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }
}
