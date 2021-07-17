use crate::client::{InnerClient, Responses};
use crate::codec::FrontendMessage;
use crate::connection::RequestMessages;
use crate::types::{BorrowToSql, Format, IsNull, Type};
use crate::{Error, Portal, Row, Statement};
use bytes::{Bytes, BytesMut};
use futures::{ready, Stream};
use log::{debug, log_enabled, Level};
use pin_project_lite::pin_project;
use postgres_protocol::message::backend::Message;
use postgres_protocol::message::frontend;
use std::fmt;
use std::marker::PhantomPinned;
use std::pin::Pin;
use std::task::{Context, Poll};

struct BorrowToSqlParamsDebug<'a, T>(&'a [T]);

impl<'a, T> fmt::Debug for BorrowToSqlParamsDebug<'a, T>
where
    T: BorrowToSql,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list()
            .entries(self.0.iter().map(|x| x.borrow_to_sql()))
            .finish()
    }
}

pub async fn query<P, I, J, K>(
    client: &InnerClient,
    statement: Statement,
    params: I,
    param_formats: J,
    column_formats: K,
) -> Result<RowStream, Error>
where
    P: BorrowToSql,
    I: IntoIterator<Item = P>,
    I::IntoIter: ExactSizeIterator,
    J: IntoIterator<Item = Format>,
    K: IntoIterator<Item = Format>,
{
    let buf = if log_enabled!(Level::Debug) {
        let params = params.into_iter().collect::<Vec<_>>();
        debug!(
            "executing statement {} with parameters: {:?}",
            statement.name(),
            BorrowToSqlParamsDebug(params.as_slice()),
        );
        encode(client, &statement, params, param_formats, column_formats)?
    } else {
        encode(client, &statement, params, param_formats, column_formats)?
    };
    let responses = start(client, buf).await?;
    Ok(RowStream {
        statement,
        responses,
        _p: PhantomPinned,
    })
}

pub async fn query_portal(
    client: &InnerClient,
    portal: &Portal,
    max_rows: i32,
) -> Result<RowStream, Error> {
    let buf = client.with_buf(|buf| {
        frontend::execute(portal.name(), max_rows, buf).map_err(Error::encode)?;
        frontend::sync(buf);
        Ok(buf.split().freeze())
    })?;

    let responses = client.send(RequestMessages::Single(FrontendMessage::Raw(buf)))?;

    Ok(RowStream {
        statement: portal.statement().clone(),
        responses,
        _p: PhantomPinned,
    })
}

pub async fn execute<P, I, J>(
    client: &InnerClient,
    statement: Statement,
    params: I,
    param_formats: J,
) -> Result<u64, Error>
where
    P: BorrowToSql,
    I: IntoIterator<Item = P>,
    I::IntoIter: ExactSizeIterator,
    J: IntoIterator<Item = Format>,
{
    let buf = if log_enabled!(Level::Debug) {
        let params = params.into_iter().collect::<Vec<_>>();
        debug!(
            "executing statement {} with parameters: {:?}",
            statement.name(),
            BorrowToSqlParamsDebug(params.as_slice()),
        );
        encode(
            client,
            &statement,
            params,
            param_formats,
            Some(Format::Binary),
        )?
    } else {
        encode(
            client,
            &statement,
            params,
            param_formats,
            Some(Format::Binary),
        )?
    };
    let mut responses = start(client, buf).await?;

    loop {
        match responses.next().await? {
            Message::DataRow(_) => {}
            Message::CommandComplete(body) => {
                let rows = body
                    .tag()
                    .map_err(Error::parse)?
                    .rsplit(' ')
                    .next()
                    .unwrap()
                    .parse()
                    .unwrap_or(0);
                return Ok(rows);
            }
            Message::EmptyQueryResponse => return Ok(0),
            _ => return Err(Error::unexpected_message()),
        }
    }
}

async fn start(client: &InnerClient, buf: Bytes) -> Result<Responses, Error> {
    let mut responses = client.send(RequestMessages::Single(FrontendMessage::Raw(buf)))?;

    match responses.next().await? {
        Message::BindComplete => {}
        _ => return Err(Error::unexpected_message()),
    }

    Ok(responses)
}

pub fn encode<P, I, J, K>(
    client: &InnerClient,
    statement: &Statement,
    params: I,
    param_formats: J,
    column_formats: K,
) -> Result<Bytes, Error>
where
    P: BorrowToSql,
    I: IntoIterator<Item = P>,
    I::IntoIter: ExactSizeIterator,
    J: IntoIterator<Item = Format>,
    K: IntoIterator<Item = Format>,
{
    client.with_buf(|buf| {
        encode_bind(statement, params, param_formats, column_formats, "", buf)?;
        frontend::execute("", 0, buf).map_err(Error::encode)?;
        frontend::sync(buf);
        Ok(buf.split().freeze())
    })
}

pub fn encode_bind<P, I, J, K>(
    statement: &Statement,
    params: I,
    param_formats: J,
    column_formats: K,
    portal: &str,
    buf: &mut BytesMut,
) -> Result<(), Error>
where
    P: BorrowToSql,
    I: IntoIterator<Item = P>,
    I::IntoIter: ExactSizeIterator,
    J: IntoIterator<Item = Format>,
    K: IntoIterator<Item = Format>,
{
    let params = params.into_iter();
    let capacity = params.len();

    assert!(
        statement.params().len() == capacity,
        "expected {} parameters but got {}",
        statement.params().len(),
        capacity,
    );

    let param_formats: Vec<Format> = param_formats.into_iter().collect();
    let column_formats = column_formats.into_iter().map(Into::into);
    let mut values = Vec::with_capacity(capacity);

    for (idx, (param, ty)) in params.zip(statement.params()).enumerate() {
        let format = param_formats.get(idx).unwrap_or(&Format::Binary);

        values.push((idx, (param, ty, format)))
    }

    let mut error_idx = 0;
    let r = frontend::bind(
        portal,
        statement.name(),
        param_formats.iter().map(Into::into),
        values,
        |(idx, (param, ty, format)), buf| {
            let ty = match format {
                Format::Binary => ty,
                Format::Text => &Type::TEXT,
            };

            match param.borrow_to_sql().to_sql_checked(ty, buf) {
                Ok(IsNull::No) => Ok(postgres_protocol::IsNull::No),
                Ok(IsNull::Yes) => Ok(postgres_protocol::IsNull::Yes),
                Err(e) => {
                    error_idx = idx;
                    Err(e)
                }
            }
        },
        column_formats,
        buf,
    );
    match r {
        Ok(()) => Ok(()),
        Err(frontend::BindError::Conversion(e)) => Err(Error::to_sql(e, error_idx)),
        Err(frontend::BindError::Serialization(e)) => Err(Error::encode(e)),
    }
}

pin_project! {
    /// A stream of table rows.
    pub struct RowStream {
        statement: Statement,
        responses: Responses,
        #[pin]
        _p: PhantomPinned,
    }
}

impl Stream for RowStream {
    type Item = Result<Row, Error>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.project();
        match ready!(this.responses.poll_next(cx)?) {
            Message::DataRow(body) => {
                Poll::Ready(Some(Ok(Row::new(this.statement.clone(), body)?)))
            }
            Message::EmptyQueryResponse
            | Message::CommandComplete(_)
            | Message::PortalSuspended => Poll::Ready(None),
            Message::ErrorResponse(body) => Poll::Ready(Some(Err(Error::db(body)))),
            _ => Poll::Ready(Some(Err(Error::unexpected_message()))),
        }
    }
}
