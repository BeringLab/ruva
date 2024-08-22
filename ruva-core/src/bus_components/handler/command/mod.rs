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

pub trait AsyncFunc<Message, T, ApplicationResult> {
	fn call(self, message: Message, t: T) -> impl std::future::Future<Output = ApplicationResult> + Send;
}
pub trait TGetHandler<R, ApplicationResult>: Sized {
	fn get_handler() -> impl AsyncFunc<Self, (R,), ApplicationResult>;
}

// impl<F, Fut, Command, Context, ApplicationResult> AsyncFunc<Command, Context, ApplicationResult> for F
// where
// 	F: Fn(Command, Context) -> Fut + Send + Clone,
// 	Fut: std::future::Future<Output = ApplicationResult> + Send,
// 	Command: crate::prelude::TCommand,
// 	Context: std::marker::Send + Sync,
// {
// 	async fn call(self, message: Command, t: Context) -> ApplicationResult {
// 		self(message, t).await
// 	}
// }

macro_rules! impl_handler {
	(
        [$($ty:ident),*]
    ) => {
		impl<F, Command, Fut,  ApplicationResult,$($ty,)*> AsyncFunc<Command, ($($ty,)*), ApplicationResult> for F
		where
			Command: crate::prelude::TCommand,

			F: Fn(Command, $($ty,)*) -> Fut + Send + Clone,
			Fut: std::future::Future<Output = ApplicationResult> + Send,
			$( $ty: Send, )*
		{
			#[allow(non_snake_case, unused_mut)]
			async fn call(self, message: Command, t: ($($ty,)*)) -> ApplicationResult {
				// destructuring tuple and count the number of elements
				// if tuple length is 0, call the function with only message and context

				let ($($ty,)*) = t;
				self(message, $($ty,)*).await
			}
		}
	};
}

all_the_tuples!(impl_handler);

#[cfg(test)]
mod tests {
	use super::*;
	use crate::prelude::TCommand;

	#[derive(Debug)]
	struct Command;
	impl TCommand for Command {}

	fn main_test<T, U>(_: T)
	where
		T: for<'a> AsyncFunc<Command, U, Result<(), ()>>,
	{
	}

	#[test]
	fn test_handler_type() {
		async fn test_fn_with_2_deps<R>(_cmd: Command, _repo: &mut R) -> Result<(), ()>
		where
			R: std::marker::Send,
		{
			Ok(())
		}

		fn test<T, C>(_: T)
		where
			T: for<'a> AsyncFunc<Command, (&'a mut C,), Result<(), ()>>,
		{
		}
		test(test_fn_with_2_deps::<crate::prelude::Context>);
		main_test(test_fn_with_2_deps::<crate::prelude::Context>);
	}

	#[test]
	fn test_handler_double_type() {
		struct SomeOtherDependency;
		async fn test_with_3_deps<R>(_cmd: Command, _repo: &mut R, _something_else: SomeOtherDependency) -> Result<(), ()>
		where
			R: std::marker::Send,
		{
			Ok(())
		}

		fn test<T, C, S>(_: T)
		where
			T: for<'a> AsyncFunc<Command, (&'a mut C, S), Result<(), ()>>,
		{
		}
		test(test_with_3_deps::<crate::prelude::Context>);
		main_test(test_with_3_deps::<crate::prelude::Context>);
	}
}
