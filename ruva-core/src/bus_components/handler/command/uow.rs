use super::*;

// all_the_tuples!(impl_uow_handler);

macro_rules! impl_uow_command_handler {
	(
        [$($ty:ident),*]
    ) => {
		impl<R, E, D1, D2,$($ty,)*> TCommandService<R, E> for CommandHandler<(D1, D2,$($ty,)*)>
		where
			R: ApplicationResponse,
			E: ApplicationError + std::convert::From<crate::responses::BaseError> + std::convert::Into<BaseError> + Clone,
			D1: TCommand + for<'a> TGetHandler<(&'a mut D2, $($ty,)*), Result<R, E>>,
			D2: TSetCurrentEvents + TUnitOfWork,
			$( $ty: Send + Sync, )*
		{
			#[allow(non_snake_case, unused_mut)]
			async fn execute(self) -> Result<R, E> {
				let (cmd, mut dep, $($ty,)* ) = self.destruct();

				dep.begin().await?;

				let result = D1::get_handler().call(cmd, (&mut dep, $($ty,)*)).await;
				match result {
					Ok(val) => {
						dep.commit().await?;
						dep.close().await;

						Ok(val)
					}
					// TODO This code only processes events that can be externally notified. Need to develop
					Err(err) => {
						dep.rollback().await?;
						dep.close().await;

						if let BaseError::StopSentinelWithEvent(event) = err.clone().into() {
							dep.set_current_events(vec![event.clone()].into());
							dep.process_internal_events().await?;
							dep.process_external_events().await?;
							Err(BaseError::StopSentinelWithEvent(event).into())
						} else {
							Err(err)
						}
					}
				}
			}
		}
	};
}

all_the_tuples!(impl_uow_command_handler);

#[macro_export]
#[doc(hidden)]
macro_rules! __register_uow_services_internal {
    // Main internal handler with optional type tuple (for extensibility)
    (

        $response:ty,
        $error:ty,
        $h:expr,
		$command:ty => $handler:expr ; [$($t:ident),*]

    ) => {
		impl<'a, > ruva::TGetHandler<(&'a mut ::ruva::Context, $($t),*), std::result::Result<$response, $error>> for $command {
			fn get_handler() -> impl ::ruva::AsyncFunc<$command, (&'a mut ::ruva::Context, $($t),*), std::result::Result<$response, $error>> {
				$handler
			}
		}

		//TODO command handler accept multiple arguments?
		impl ::ruva::TMessageBus<$response, $error, $command> for ::ruva::DefaultMessageBus
		{
			fn command_handler(
				&self,
				cmd: $command,
				context_manager: ruva::AtomicContextManager,
			) -> impl ::ruva::TCommandService<$response, $error> {

				$h(::ruva::CommandHandler((
					cmd,
					::ruva::Context::new(context_manager),
					$(
						$t::reflect(),
					)*
					)
				))
			}
		}

    };
}

#[macro_export]
macro_rules! register_uow_services {

    // Case with custom handler function
    (
        $response:ty,
        $error:ty,
        $h:expr,
        $(
            $command:ty => $handler:expr
        ),*
    ) => {
		$(
			::ruva::__register_uow_services_internal!($response, $error, $h, $command => $handler;[]);
		)*


    };

    // Default case with custom bus
    (
        $response:ty,
        $error:ty,

        $(
            $command:ty => $handler:expr
        ),*
    ) => {
		$(
			::ruva::__register_uow_services_internal!($response, $error, ::std::convert::identity, $command => $handler;[]);
		)*
    };
}
