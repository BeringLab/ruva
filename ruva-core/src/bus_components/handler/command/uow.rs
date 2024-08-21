use super::*;

// all_the_tuples!(impl_uow_handler);

// tuple expression!
impl<R, E, D1, D2> TCommandService<R, E> for CommandHandler<(D1, D2)>
where
	R: ApplicationResponse,
	E: ApplicationError + std::convert::From<crate::responses::BaseError> + std::convert::Into<BaseError> + Clone,
	D1: TCommand + for<'a> TGetHandler<&'a mut D2, Result<R, E>>,
	D2: TSetCurrentEvents + TUnitOfWork,
{
	async fn execute(self) -> Result<R, E> {
		let (cmd, mut dep) = self.destruct();

		dep.begin().await?;

		let result = D1::get_handler().call(cmd, &mut dep).await;
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

#[macro_export]
#[doc(hidden)]
macro_rules! __register_uow_services_internal {
    (
        $messagebus:ty,
        $response:ty,
        $error:ty,
        $h:expr,

        $(
            $command:ty => $handler:expr
        ),*
    ) => {
        type ApplicationResult = std::result::Result<$response,$error>;

        $(
            impl<'a> ruva::TGetHandler<&'a mut ::ruva::Context, ApplicationResult> for $command {
                fn get_handler() -> impl ::ruva::AsyncFunc<$command, &'a mut ::ruva::Context, ApplicationResult > {
                    $handler
                }
            }

            impl ::ruva::TMessageBus<$response,$error,$command> for $messagebus{
                fn command_handler(
                    &self,
                    context_manager: ruva::AtomicContextManager,
                    cmd: $command,
                ) -> impl ::ruva::TCommandService<$response, $error> {
                    $h(::ruva::CommandHandler((cmd, ::ruva::Context::new(context_manager))))
                }
            }
        )*
    };
}

#[macro_export]
macro_rules! register_uow_services {
    // Case with custom handler function
    (
        $messagebus:ty,
        $response:ty,
        $error:ty,
        $h:expr,

        $(
            $command:ty => $handler:expr
        ),*
    ) => {
       	ruva::__register_uow_services_internal!($messagebus, $response, $error, $h, $($command => $handler),*);
    };

    // Default case
    (
        $messagebus:ty,
        $response:ty,
        $error:ty,

        $(
            $command:ty => $handler:expr
        ),*
    ) => {
        ruva::__register_uow_services_internal!($messagebus, $response, $error, ::std::convert::identity, $($command => $handler),*);
    };
}
