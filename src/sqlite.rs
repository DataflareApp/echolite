use crate::{Error, Result};
use protocol::{Column, Flags, Query, Value};
use rusqlite::{Connection, OpenFlags, types::ValueRef};
use std::time::Instant;

#[derive(Debug)]
pub struct Sqlite {
    conn: Connection,
}

impl Sqlite {
    pub fn connect(path: &str, flags: Flags) -> Result<Self> {
        let open = OpenFlags::from_bits(flags.bits()).ok_or_else(|| Error::InvalidFlags)?;
        let conn = Connection::open_with_flags(path, open)?;
        Ok(Self { conn })
    }

    pub fn query(&self, sql: &str) -> Result<Query> {
        let t = Instant::now();
        let mut stmt = self.conn.prepare(sql)?;

        let columns = stmt
            .columns()
            .into_iter()
            .map(|col| Column {
                name: col.name().into(),
                datatype: col.decl_type().unwrap_or_default().into(),
            })
            .collect::<Vec<_>>();

        let mut rows = stmt.query([])?;
        let mut values = Vec::new();
        while let Some(row) = rows.next()? {
            for i in 0..columns.len() {
                let v = row.get_ref(i)?;
                let v = match v {
                    ValueRef::Null => Value::Null,
                    ValueRef::Integer(i) => Value::I64(i),
                    ValueRef::Real(f) => Value::F64(f),
                    ValueRef::Text(s) => Value::Text(s.to_vec()),
                    ValueRef::Blob(b) => Value::Bytes(b.to_vec()),
                };
                values.push(v);
            }
        }

        Ok(Query {
            columns,
            values,
            rows_affected: self.conn.changes(),
            duration: t.elapsed().as_millis() as u64,
        })
    }

    pub fn execute(&self, sql: &str) -> Result<()> {
        self.conn.execute_batch(sql)?;
        Ok(())
    }

    pub fn transaction(&mut self, sqls: Vec<String>) -> Result<()> {
        if sqls.is_empty() {
            return Ok(());
        }
        let tx = self.conn.transaction()?;
        for sql in sqls {
            tx.execute(&sql, ())?;
        }
        tx.commit()?;
        Ok(())
    }
}
