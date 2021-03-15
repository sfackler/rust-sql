use crate::query::RowStream;
use crate::types::{BorrowToSql, Format, ToSql, Type};
use crate::{Client, Error, Row, Statement, ToStatement, Transaction};
use async_trait::async_trait;

mod private {
    pub trait Sealed {}
}

/// A trait allowing abstraction over connections and transactions.
///
/// This trait is "sealed", and cannot be implemented outside of this crate.
#[async_trait]
pub trait GenericClient: private::Sealed {
    /// Like `Client::execute`.
    async fn execute<T>(&self, query: &T, params: &[&(dyn ToSql + Sync)]) -> Result<u64, Error>
    where
        T: ?Sized + ToStatement + Sync + Send;

    /// Like `Client::execute_raw`.
    async fn execute_raw<P, I, J, T>(
        &self,
        statement: &T,
        params: I,
        param_formats: J,
    ) -> Result<u64, Error>
    where
        T: ?Sized + ToStatement + Sync + Send,
        P: BorrowToSql,
        I: IntoIterator<Item = P> + Sync + Send,
        I::IntoIter: ExactSizeIterator,
        J: IntoIterator<Item = Format> + Sync + Send;

    /// Like `Client::query`.
    async fn query<T>(&self, query: &T, params: &[&(dyn ToSql + Sync)]) -> Result<Vec<Row>, Error>
    where
        T: ?Sized + ToStatement + Sync + Send;

    /// Like `Client::query_one`.
    async fn query_one<T>(
        &self,
        statement: &T,
        params: &[&(dyn ToSql + Sync)],
    ) -> Result<Row, Error>
    where
        T: ?Sized + ToStatement + Sync + Send;

    /// Like `Client::query_opt`.
    async fn query_opt<T>(
        &self,
        statement: &T,
        params: &[&(dyn ToSql + Sync)],
    ) -> Result<Option<Row>, Error>
    where
        T: ?Sized + ToStatement + Sync + Send;

    /// Like `Client::query_raw`.
    async fn query_raw<T, P, I, J, K>(
        &self,
        statement: &T,
        params: I,
        param_formats: J,
        column_formats: K,
    ) -> Result<RowStream, Error>
    where
        T: ?Sized + ToStatement + Sync + Send,
        P: BorrowToSql,
        I: IntoIterator<Item = P> + Sync + Send,
        I::IntoIter: ExactSizeIterator,
        J: IntoIterator<Item = Format> + Sync + Send,
        K: IntoIterator<Item = Format> + Sync + Send;

    /// Like `Client::prepare`.
    async fn prepare(&self, query: &str) -> Result<Statement, Error>;

    /// Like `Client::prepare_typed`.
    async fn prepare_typed(
        &self,
        query: &str,
        parameter_types: &[Type],
    ) -> Result<Statement, Error>;

    /// Like `Client::transaction`.
    async fn transaction(&mut self) -> Result<Transaction<'_>, Error>;
}

impl private::Sealed for Client {}

#[async_trait]
impl GenericClient for Client {
    async fn execute<T>(&self, query: &T, params: &[&(dyn ToSql + Sync)]) -> Result<u64, Error>
    where
        T: ?Sized + ToStatement + Sync + Send,
    {
        self.execute(query, params).await
    }

    async fn execute_raw<P, I, J, T>(
        &self,
        statement: &T,
        params: I,
        param_formats: J,
    ) -> Result<u64, Error>
    where
        T: ?Sized + ToStatement + Sync + Send,
        P: BorrowToSql,
        I: IntoIterator<Item = P> + Sync + Send,
        I::IntoIter: ExactSizeIterator,
        J: IntoIterator<Item = Format> + Sync + Send,
    {
        self.execute_raw(statement, params, param_formats).await
    }

    async fn query<T>(&self, query: &T, params: &[&(dyn ToSql + Sync)]) -> Result<Vec<Row>, Error>
    where
        T: ?Sized + ToStatement + Sync + Send,
    {
        self.query(query, params).await
    }

    async fn query_one<T>(
        &self,
        statement: &T,
        params: &[&(dyn ToSql + Sync)],
    ) -> Result<Row, Error>
    where
        T: ?Sized + ToStatement + Sync + Send,
    {
        self.query_one(statement, params).await
    }

    async fn query_opt<T>(
        &self,
        statement: &T,
        params: &[&(dyn ToSql + Sync)],
    ) -> Result<Option<Row>, Error>
    where
        T: ?Sized + ToStatement + Sync + Send,
    {
        self.query_opt(statement, params).await
    }

    async fn query_raw<T, P, I, J, K>(
        &self,
        statement: &T,
        params: I,
        param_formats: J,
        column_formats: K,
    ) -> Result<RowStream, Error>
    where
        T: ?Sized + ToStatement + Sync + Send,
        P: BorrowToSql,
        I: IntoIterator<Item = P> + Sync + Send,
        I::IntoIter: ExactSizeIterator,
        J: IntoIterator<Item = Format> + Sync + Send,
        K: IntoIterator<Item = Format> + Sync + Send,
    {
        self.query_raw(statement, params, param_formats, column_formats)
            .await
    }

    async fn prepare(&self, query: &str) -> Result<Statement, Error> {
        self.prepare(query).await
    }

    async fn prepare_typed(
        &self,
        query: &str,
        parameter_types: &[Type],
    ) -> Result<Statement, Error> {
        self.prepare_typed(query, parameter_types).await
    }

    async fn transaction(&mut self) -> Result<Transaction<'_>, Error> {
        self.transaction().await
    }
}

impl private::Sealed for Transaction<'_> {}

#[async_trait]
#[allow(clippy::needless_lifetimes)]
impl GenericClient for Transaction<'_> {
    async fn execute<T>(&self, query: &T, params: &[&(dyn ToSql + Sync)]) -> Result<u64, Error>
    where
        T: ?Sized + ToStatement + Sync + Send,
    {
        self.execute(query, params).await
    }

    async fn execute_raw<P, I, J, T>(
        &self,
        statement: &T,
        params: I,
        param_formats: J,
    ) -> Result<u64, Error>
    where
        T: ?Sized + ToStatement + Sync + Send,
        P: BorrowToSql,
        I: IntoIterator<Item = P> + Sync + Send,
        I::IntoIter: ExactSizeIterator,
        J: IntoIterator<Item = Format> + Sync + Send,
    {
        self.execute_raw(statement, params, param_formats).await
    }

    async fn query<T>(&self, query: &T, params: &[&(dyn ToSql + Sync)]) -> Result<Vec<Row>, Error>
    where
        T: ?Sized + ToStatement + Sync + Send,
    {
        self.query(query, params).await
    }

    async fn query_one<T>(
        &self,
        statement: &T,
        params: &[&(dyn ToSql + Sync)],
    ) -> Result<Row, Error>
    where
        T: ?Sized + ToStatement + Sync + Send,
    {
        self.query_one(statement, params).await
    }

    async fn query_opt<T>(
        &self,
        statement: &T,
        params: &[&(dyn ToSql + Sync)],
    ) -> Result<Option<Row>, Error>
    where
        T: ?Sized + ToStatement + Sync + Send,
    {
        self.query_opt(statement, params).await
    }

    async fn query_raw<T, P, I, J, K>(
        &self,
        statement: &T,
        params: I,
        param_formats: J,
        column_formats: K,
    ) -> Result<RowStream, Error>
    where
        T: ?Sized + ToStatement + Sync + Send,
        P: BorrowToSql,
        I: IntoIterator<Item = P> + Sync + Send,
        I::IntoIter: ExactSizeIterator,
        J: IntoIterator<Item = Format> + Sync + Send,
        K: IntoIterator<Item = Format> + Sync + Send,
    {
        self.query_raw(statement, params, param_formats, column_formats)
            .await
    }

    async fn prepare(&self, query: &str) -> Result<Statement, Error> {
        self.prepare(query).await
    }

    async fn prepare_typed(
        &self,
        query: &str,
        parameter_types: &[Type],
    ) -> Result<Statement, Error> {
        self.prepare_typed(query, parameter_types).await
    }

    #[allow(clippy::needless_lifetimes)]
    async fn transaction<'a>(&'a mut self) -> Result<Transaction<'a>, Error> {
        self.transaction().await
    }
}
