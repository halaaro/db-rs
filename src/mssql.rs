use anyhow::Context;
use async_std::net::TcpStream;

type Result<T> = std::result::Result<T, anyhow::Error>;

pub(crate) struct Connection {
    _config: tiberius::Config,
    client: tiberius::Client<TcpStream>,
}

pub(crate) struct QueryBuilder<'a>(tiberius::Query<'a>);

impl Connection {
    pub(crate) async fn from_string(conn_str: &str) -> Result<Self> {
        let config =
            tiberius::Config::from_ado_string(conn_str).context("connection string invalid")?;
        // TODO: implement retries, connection pooling
        let tcp = TcpStream::connect(config.get_addr()).await?;
        tcp.set_nodelay(true)?;
        let client = tiberius::Client::connect(config.clone(), tcp).await?;

        Ok(Self {
            _config: config,
            client,
        })
    }
}

impl<'a> QueryBuilder<'a> {
    pub(crate) fn new(query_string: &'a str) -> Self {
        Self(tiberius::Query::new(query_string))
    }
    pub(crate) async fn execute(self, connection: &mut Connection) -> Result<QueryResults> {
        let stream = self.0.query(&mut connection.client).await?;
        let results = stream.into_results().await?;
        Ok(QueryResults::new(results))
    }
}

pub(crate) struct QueryResults(Vec<Vec<tiberius::Row>>);

impl QueryResults {
    fn new(results: Vec<Vec<tiberius::Row>>) -> Self {
        Self(results)
    }
}

pub(crate) struct ResultSetIter(Vec<Vec<tiberius::Row>>);

impl IntoIterator for QueryResults {
    type Item = ResultSet;
    type IntoIter = ResultSetIter;

    fn into_iter(mut self) -> Self::IntoIter {
        self.0.reverse();
        ResultSetIter(self.0)
    }
}

impl Iterator for ResultSetIter {
    type Item = ResultSet;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.pop().map(ResultSet)
    }
}

#[derive(Debug)]
pub(crate) struct ResultSet(Vec<tiberius::Row>);

impl ResultSet {
    pub(crate) fn into_json_fmt(self) -> json::JsonFmtResultSet {
        json::JsonFmtResultSet(self.into_iter().collect())
    }
}

pub(crate) struct ResultRowIter(Vec<tiberius::Row>);

impl IntoIterator for ResultSet {
    type Item = ResultRow;
    type IntoIter = ResultRowIter;

    fn into_iter(mut self) -> Self::IntoIter {
        self.0.reverse();
        ResultRowIter(self.0)
    }
}

impl Iterator for ResultRowIter {
    type Item = ResultRow;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.pop().map(ResultRow)
    }
}
pub(crate) struct ResultRow(tiberius::Row);

struct ResultValueIter<'a> {
    idx: usize,
    len: usize,
    row: &'a ResultRow,
}

impl<'a> Iterator for ResultValueIter<'a> {
    type Item = ResultValue<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.idx >= self.len {
            return None;
        }
        let val = self.row.value(self.idx);
        self.idx += 1;
        val
    }
}

#[derive(Debug, Clone)]
pub(crate) struct ResultValueOwned(tiberius::ColumnData<'static>);

impl IntoIterator for ResultRow {
    type Item = ResultValueOwned;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.0
            .into_iter()
            .map(ResultValueOwned)
            .collect::<Vec<_>>()
            .into_iter()
    }
}

impl<'a> ResultRow {
    pub(crate) fn iter_columns(&'a self) -> impl Iterator<Item = &'a str> {
        self.0.columns().iter().map(|c| c.name())
    }

    pub(crate) fn value(&'a self, index: usize) -> Option<ResultValue<'a>> {
        self.0.get(index)
    }

    pub(crate) fn iter_values(&'a self) -> impl Iterator<Item = ResultValue<'a>> {
        ResultValueIter {
            idx: 0,
            len: self.0.len(),
            row: self,
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct ResultValue<'a>(tiberius::ColumnData<'a>);

impl<'a> tiberius::FromSql<'a> for ResultValue<'a> {
    fn from_sql(value: &'a tiberius::ColumnData<'static>) -> tiberius::Result<Option<Self>> {
        Ok(Some(ResultValue(value.clone()))) // FIXME: avoid clone here
    }
}

// newtype pattern
struct ResultValueRef<'a, 'b>(&'a tiberius::ColumnData<'b>);

pub(crate) mod fmt {
    use std::fmt::Display;

    use chrono::{DateTime, FixedOffset, NaiveDate, NaiveDateTime, NaiveTime};

    use super::{ResultValue, ResultValueOwned, ResultValueRef};

    impl<'a, 'b> Display for ResultValueRef<'a, 'b> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            type C<'a> = tiberius::ColumnData<'a>;
            match &self.0 {
                C::U8(u) => fmt_std(f, u),
                C::I16(i) => fmt_std(f, i),
                C::I32(i) => fmt_std(f, i),
                C::I64(i) => fmt_std(f, i),
                C::F32(n) => fmt_std(f, n),
                C::F64(n) => fmt_std(f, n),
                C::Bit(b) => fmt_std(f, b),
                C::String(s) => fmt_str(f, s),
                C::Guid(g) => fmt_std(f, g),
                C::Binary(b) => fmt_hex(f, b),
                C::Numeric(n) => fmt_std(f, n),
                C::Xml(x) => fmt_xml(f, x),
                C::DateTime(d) => fmt_date(f, DateFormat::DateTime(d)),
                C::SmallDateTime(d) => fmt_date(f, DateFormat::SmallDateTime(d)),
                C::Time(d) => fmt_date(f, DateFormat::Time(d)),
                C::Date(d) => fmt_date(f, DateFormat::Date(d)),
                C::DateTime2(d) => fmt_date(f, DateFormat::DateTime2(d)),
                C::DateTimeOffset(d) => fmt_date(f, DateFormat::DateTimeOffset(d)),
            }
        }
    }

    impl<'a> Display for ResultValue<'a> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            ResultValueRef(&self.0).fmt(f)
        }
    }

    impl Display for ResultValueOwned {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            ResultValueRef(&self.0).fmt(f)
        }
    }

    enum DateFormat<'a> {
        DateTimeOffset(&'a Option<tiberius::time::DateTimeOffset>),
        DateTime2(&'a Option<tiberius::time::DateTime2>),
        DateTime(&'a Option<tiberius::time::DateTime>),
        SmallDateTime(&'a Option<tiberius::time::SmallDateTime>),
        Time(&'a Option<tiberius::time::Time>),
        Date(&'a Option<tiberius::time::Date>),
    }

    fn fmt_date(
        f: &mut std::fmt::Formatter,
        d: DateFormat,
    ) -> std::result::Result<(), std::fmt::Error> {
        match d {
            DateFormat::DateTimeOffset(d) => {
                let d = tiberius::ColumnData::DateTimeOffset(*d);
                let dt = <DateTime<FixedOffset> as tiberius::FromSql>::from_sql(&d);
                match dt {
                    Ok(Some(dt)) => write!(f, "{}", dt.format("%+")),
                    _ => fmt_null(f),
                }
            }
            DateFormat::DateTime2(d) => {
                let d = tiberius::ColumnData::DateTime2(*d);
                let dt = <NaiveDateTime as tiberius::FromSql>::from_sql(&d);
                match dt {
                    Ok(Some(dt)) => write!(f, "{}", dt.format("%Y-%m-%dT%H:%M:%S%.7f")),
                    _ => fmt_null(f),
                }
            }
            DateFormat::DateTime(d) => {
                let d = tiberius::ColumnData::DateTime(*d);
                let dt = <NaiveDateTime as tiberius::FromSql>::from_sql(&d);
                match dt {
                    Ok(Some(dt)) => write!(f, "{}", dt.format("%Y-%m-%dT%H:%M:%S%.3f")),
                    _ => fmt_null(f),
                }
            }
            DateFormat::SmallDateTime(d) => {
                let d = tiberius::ColumnData::SmallDateTime(*d);
                let dt = <NaiveDateTime as tiberius::FromSql>::from_sql(&d);
                match dt {
                    Ok(Some(dt)) => write!(f, "{}", dt.format("%Y-%m-%dT%H:%M:%S")),
                    _ => fmt_null(f),
                }
            }
            DateFormat::Time(t) => {
                let d = tiberius::ColumnData::Time(*t);
                let dt = <NaiveTime as tiberius::FromSql>::from_sql(&d);
                match dt {
                    Ok(Some(dt)) => write!(f, "{}", dt.format("%H:%M:%S%.3f")),
                    _ => fmt_null(f),
                }
            }
            DateFormat::Date(d) => {
                let d = tiberius::ColumnData::Date(*d);
                let dt = <NaiveDate as tiberius::FromSql>::from_sql(&d);
                match dt {
                    Ok(Some(dt)) => write!(f, "{}", dt.format("%Y-%m-%d")),
                    _ => fmt_null(f),
                }
            }
        }
    }

    fn fmt_xml(
        f: &mut std::fmt::Formatter,
        x: &Option<impl AsRef<tiberius::xml::XmlData>>,
    ) -> std::result::Result<(), std::fmt::Error> {
        match x {
            Some(x) => fmt_str(f, &Some((x as &dyn AsRef<tiberius::xml::XmlData>).as_ref())),
            None => fmt_null(f),
        }
    }

    fn fmt_std(
        f: &mut std::fmt::Formatter,
        val: &Option<impl Display>,
    ) -> std::result::Result<(), std::fmt::Error> {
        match val {
            Some(i) => i.fmt(f),
            None => fmt_null(f),
        }
    }

    fn fmt_null(f: &mut std::fmt::Formatter) -> std::result::Result<(), std::fmt::Error> {
        write!(f, "null")
    }

    fn fmt_str(
        f: &mut std::fmt::Formatter,
        s: &Option<impl AsRef<str>>,
    ) -> std::result::Result<(), std::fmt::Error> {
        let width = f.width().unwrap_or(usize::MAX);
        match s {
            Some(s) => {
                let (s, postfix) = if s.as_ref().len() > width - 2 {
                    (&s.as_ref()[..width - 3], "...")
                } else {
                    (s.as_ref(), "")
                };
                write!(f, "{s}{postfix}")
            }
            None => fmt_null(f),
        }
    }

    fn fmt_hex(
        f: &mut std::fmt::Formatter,
        b: &Option<impl AsRef<[u8]>>,
    ) -> std::result::Result<(), std::fmt::Error> {
        let width = f.width().unwrap_or(usize::MAX);
        match b {
            Some(b) => {
                let (b, postfix) = if b.as_ref().len() > width {
                    (&b.as_ref()[..width / 2 - 2], "...")
                } else {
                    (b.as_ref(), "")
                };
                for bi in b {
                    write!(f, "{bi:x}")?
                }
                write!(f, "{postfix}")
            }
            None => fmt_null(f),
        }
    }
}

pub(crate) mod json {
    use std::fmt::Display;

    use super::{ResultRow, ResultValue};

    pub(crate) struct JsonFmtResultSet(pub(crate) Vec<ResultRow>);
    pub(crate) struct JsonFmtResultRow<'a>(&'a ResultRow);
    pub(crate) struct JsonFmtResultColumn<'a>(&'a str);
    pub(crate) struct JsonFmtResultValue<'a>(ResultValue<'a>);

    impl Display for JsonFmtResultSet {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "[")?;
            let len = self.0.len();
            for (i, row) in self.0.iter().enumerate() {
                write!(f, "{}", JsonFmtResultRow(row))?;
                if i < len - 1 {
                    write!(f, ",")?;
                }
            }
            writeln!(f, "]")
        }
    }
    impl<'a> Display for JsonFmtResultRow<'a> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            let cols = self.0.iter_columns();
            let len = self.0 .0.len();
            write!(f, "{{")?;
            for (i, (col, val)) in cols.zip(self.0.iter_values()).enumerate() {
                write!(
                    f,
                    "\"{}\": {}",
                    JsonFmtResultColumn(col),
                    JsonFmtResultValue(val)
                )?;
                if i < len - 1 {
                    write!(f, ",")?;
                }
            }
            write!(f, "}}")
        }
    }

    impl<'a> Display for JsonFmtResultColumn<'a> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            for c in self.0.chars() {
                match c {
                    '\0' | '"' | '\\' | '\t' | '\n' | '\r' => write!(f, "{}", c.escape_default()),
                    '\u{0001}'..='\u{001f}' => {
                        let mut buf = [0; 2];
                        c.encode_utf16(&mut buf);
                        write!(f, "\\u{:02x}{:02x}", buf[0] >> 8, buf[0] & 0xFF)
                    }
                    _ => write!(f, "{c}"),
                }?;
            }
            Ok(())
        }
    }

    impl<'a> Display for JsonFmtResultValue<'a> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            type C<'a> = tiberius::ColumnData<'a>;
            match self.0 .0 {
                // TODO: add escaping when needed
                C::Guid(Some(_))
                | C::Binary(Some(_))
                | C::String(Some(_))
                | C::Xml(Some(_))
                | C::DateTime(Some(_))
                | C::SmallDateTime(Some(_))
                | C::Time(Some(_))
                | C::Date(Some(_))
                | C::DateTime2(Some(_))
                | C::DateTimeOffset(Some(_)) => write!(f, "\"{}\"", self.0),
                _ => write!(f, "{}", self.0),
            }
        }
    }
}
