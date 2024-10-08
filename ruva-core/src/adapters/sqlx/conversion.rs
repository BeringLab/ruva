use crate::prelude::BaseError;
use crate::snowflake::SnowFlake;

use sqlx::error::BoxDynError;
use sqlx::postgres::{PgHasArrayType, PgTypeInfo, PgValueRef};
use sqlx::{Encode, Postgres, Type};

impl Encode<'_, Postgres> for SnowFlake {
	fn encode_by_ref(&self, buf: &mut <Postgres as sqlx::Database>::ArgumentBuffer<'_>) -> Result<sqlx::encode::IsNull, BoxDynError> {
		let value = self.0;
		<i64 as Encode<Postgres>>::encode(value, buf)
	}
}

impl<'r> sqlx::Decode<'r, Postgres> for SnowFlake {
	fn decode(value: PgValueRef<'r>) -> Result<Self, BoxDynError> {
		let i64_value = <i64 as sqlx::Decode<Postgres>>::decode(value)?;
		Ok(SnowFlake(i64_value))
	}
}

impl sqlx::Type<Postgres> for SnowFlake {
	fn type_info() -> sqlx::postgres::PgTypeInfo {
		<i64 as Type<Postgres>>::type_info()
	}

	fn compatible(ty: &PgTypeInfo) -> bool {
		<i64 as Type<Postgres>>::compatible(ty)
	}
}

impl From<sqlx::Error> for BaseError {
	fn from(value: sqlx::Error) -> Self {
		tracing::error!("{:?}", value);
		Self::DatabaseError(value.to_string())
	}
}

impl PgHasArrayType for SnowFlake {
	fn array_type_info() -> PgTypeInfo {
		<i64 as PgHasArrayType>::array_type_info()
	}
}
