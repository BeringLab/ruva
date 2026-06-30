use crate::bus_components::contexts::Context;
use crate::{
	prelude::{BaseError, TUnitOfWork},
	prepare_bulk_operation,
};
use sqlx::{PgConnection, PgPool};
use std::sync::OnceLock;

const SERVICE_OUTBOX_STATE_COLUMN: &str = "state";
const SERVICE_OUTBOX_EVENT_PAYLOAD_COLUMN: &str = "event_payload";
static SERVICE_OUTBOX_PAYLOAD_COLUMN_CACHE: OnceLock<&'static str> = OnceLock::new();

fn choose_service_outbox_payload_column(columns: &[String]) -> Option<&'static str> {
	if columns.iter().any(|column| column == SERVICE_OUTBOX_STATE_COLUMN) {
		Some(SERVICE_OUTBOX_STATE_COLUMN)
	} else if columns.iter().any(|column| column == SERVICE_OUTBOX_EVENT_PAYLOAD_COLUMN) {
		Some(SERVICE_OUTBOX_EVENT_PAYLOAD_COLUMN)
	} else {
		None
	}
}

fn service_outbox_insert_query(payload_column: &str) -> String {
	format!(
		r#"
            INSERT INTO service_outbox
                (id, aggregate_id, topic, {payload_column}, aggregate_name, trace_id)
            SELECT * FROM UNNEST
                ($1::BIGINT[], $2::text[],  $3::text[], $4::text[], $5::text[], $6::text[])
            "#
	)
}

async fn service_outbox_payload_column(conn: &mut PgConnection) -> Result<&'static str, BaseError> {
	if let Some(column) = SERVICE_OUTBOX_PAYLOAD_COLUMN_CACHE.get().copied() {
		return Ok(column);
	}

	let columns = sqlx::query_scalar::<_, String>(
		r#"
            SELECT attname
            FROM pg_attribute
            WHERE attrelid = to_regclass('service_outbox')
              AND attname IN ('state', 'event_payload')
              AND NOT attisdropped
            "#,
	)
	.fetch_all(conn)
	.await
	.map_err(|err| {
		tracing::error!("failed to inspect service_outbox payload column! {}", err);
		BaseError::DatabaseError(err.to_string())
	})?;

	let column = choose_service_outbox_payload_column(&columns).ok_or_else(|| {
		let message = "service_outbox requires either state or event_payload column".to_string();
		tracing::error!("{}", message);
		BaseError::DatabaseError(message)
	})?;

	let _ = SERVICE_OUTBOX_PAYLOAD_COLUMN_CACHE.set(column);
	Ok(SERVICE_OUTBOX_PAYLOAD_COLUMN_CACHE.get().copied().unwrap_or(column))
}

impl Context {
	pub fn transaction(&mut self) -> &mut PgConnection {
		match self.pg_transaction.as_mut() {
			Some(trx) => trx,
			None => panic!("Transaction Has Not Begun!"),
		}
	}

	pub(crate) async fn save_outbox(&mut self) -> Result<(), BaseError> {
		let outboxes = self.curr_events.iter().filter(|e| e.externally_notifiable()).map(|o| o.outbox()).collect::<Vec<_>>();

		if outboxes.is_empty() {
			return Ok(());
		}

		prepare_bulk_operation!(
			&outboxes,
			id: i64,
			aggregate_id: String,
			aggregate_name:String,
			topic: String,
			state: String,
			trace_id: String
		);
		let payload_column = service_outbox_payload_column(self.transaction()).await?;
		let query = service_outbox_insert_query(payload_column);

		sqlx::query(&query)
			.bind(&id)
			.bind(&aggregate_id)
			.bind(&topic)
			.bind(&state)
			.bind(&aggregate_name)
			.bind(&trace_id)
			.execute(self.transaction())
			.await
			.map_err(|err| {
				tracing::error!("failed to insert outbox! {}", err);
				BaseError::DatabaseError(err.to_string())
			})?;
		Ok(())
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn chooses_state_when_state_column_exists() {
		let columns = vec!["state".to_string(), "event_payload".to_string()];

		assert_eq!(choose_service_outbox_payload_column(&columns), Some("state"));
	}

	#[test]
	fn chooses_event_payload_when_state_column_is_missing() {
		let columns = vec!["event_payload".to_string()];

		assert_eq!(choose_service_outbox_payload_column(&columns), Some("event_payload"));
	}

	#[test]
	fn rejects_unknown_payload_columns() {
		let columns = vec!["payload".to_string()];

		assert_eq!(choose_service_outbox_payload_column(&columns), None);
	}

	#[test]
	fn insert_query_uses_selected_payload_column() {
		let query = service_outbox_insert_query("event_payload");

		assert!(query.contains("(id, aggregate_id, topic, event_payload, aggregate_name, trace_id)"));
	}
}

impl TUnitOfWork for Context {
	async fn begin(&mut self) -> Result<(), BaseError> {
		match self.pg_transaction.as_mut() {
			None => {
				let trx = self.super_ctx.conn;

				if let Some(trx) = trx.downcast_ref::<&PgPool>().or(trx.downcast_ref::<PgPool>().as_ref()) {
					self.pg_transaction = Some(trx.begin().await?);
				} else {
					tracing::error!("Transaction Error!");
					return Err(BaseError::TransactionError);
				}
				// simplify above

				Ok(())
			}
			Some(_trx) => {
				tracing::warn!("Transaction Begun Already!");
				Err(BaseError::TransactionError)?
			}
		}
	}

	async fn _commit(&mut self) -> Result<(), BaseError> {
		match self.pg_transaction.take() {
			None => panic!("Tranasction Has Not Begun!"),
			Some(trx) => Ok(trx.commit().await?),
		}
	}

	async fn rollback(&mut self) -> Result<(), BaseError> {
		self.curr_events.clear();
		match self.pg_transaction.take() {
			None => panic!("Tranasction Has Not Begun!"),
			Some(trx) => Ok(trx.rollback().await?),
		}
	}
	async fn close(&mut self) {
		match self.pg_transaction.take() {
			None => (),
			Some(trx) => {
				let _ = trx.rollback().await;
			}
		}
	}

	async fn process_internal_events(&mut self) -> Result<(), BaseError> {
		self.send_internally_notifiable_messages().await;
		Ok(())
	}

	async fn process_external_events(&mut self) -> Result<(), BaseError> {
		self.save_outbox().await?;
		Ok(())
	}
}
