//! ### Example - simple command handler
//! ```rust,no_run
//! impl<C,R> TCommandService<(), ()> for CommandHandler<(C, R)>
//! where
//!     C: crate::prelude::TCommand + for<'a> TGetHandler<&'a mut R, Result<(), ()>>,
//!     R: Send + Sync,
//! {
//!     async fn execute(mut self) -> Result<(), ()> {
//!         let CommandHandler((cmd, mut dep)) = self;
//!         let handler = C::get_handler();
//!         handler(cmd, &mut dep).await
//!     }
//! }
//! ```

pub mod uow;
use crate::{
	message::TCommand,
	prelude::{ApplicationError, ApplicationResponse, BaseError, TCommandService, TSetCurrentEvents, TUnitOfWork},
};

pub struct CommandHandler<T>(pub T);

impl<T> CommandHandler<T> {
	pub fn destruct(self) -> T {
		self.0
	}
}

pub trait AsyncFunc<Message, Context, ApplicationResult> {
	type Future: std::future::Future<Output = ApplicationResult> + Send;
	fn call(self, message: Message, context: Context) -> Self::Future;
}

impl<F, Command, Fut, Context, ApplicationResult> AsyncFunc<Command, Context, ApplicationResult> for F
where
	Command: crate::prelude::TCommand,
	Context: std::marker::Send + Sync + 'static,
	F: Fn(Command, Context) -> Fut + Send + Clone + 'static,
	Fut: std::future::Future<Output = ApplicationResult> + Send,
{
	type Future = std::pin::Pin<Box<dyn std::future::Future<Output = ApplicationResult> + Send>>;

	fn call(self, message: Command, context: Context) -> Self::Future {
		Box::pin(async move { self(message, context).await })
	}
}

pub trait TGetHandler<R, ApplicationResult>: Sized {
	fn get_handler() -> impl AsyncFunc<Self, R, ApplicationResult>;
}
