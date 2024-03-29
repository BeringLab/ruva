use crate::prelude::{TCommand, TCommandService, TEvent};
use crate::responses::{self, ApplicationError, ApplicationResponse, BaseError};
use async_trait::async_trait;
use hashbrown::HashMap;
use std::collections::VecDeque;
use std::ops::{Deref, DerefMut};
use std::{pin::Pin, sync::Arc};
use tokio::sync::RwLock;

pub type Future<T, E> = Pin<Box<dyn futures::Future<Output = Result<T, E>> + Send>>;
pub type AtomicContextManager = Arc<RwLock<ContextManager>>;
pub type TEventHandler<R, E> = HashMap<String, Vec<Box<dyn Fn(std::sync::Arc<dyn TEvent>, AtomicContextManager) -> Future<R, E> + Send + Sync>>>;

/// Task Local Context Manager
/// This is called for every time `handle` method is invoked.
pub struct ContextManager {
	pub event_queue: VecDeque<Arc<dyn TEvent>>,
}

impl ContextManager {
	/// Creation of context manager returns context manager AND event receiver
	pub fn new() -> AtomicContextManager {
		Arc::new(RwLock::new(Self { event_queue: VecDeque::new() }))
	}
}

impl Deref for ContextManager {
	type Target = VecDeque<Arc<dyn TEvent>>;
	fn deref(&self) -> &Self::Target {
		&self.event_queue
	}
}
impl DerefMut for ContextManager {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.event_queue
	}
}

#[async_trait]
pub trait TEventBus<R, E>
where
	R: ApplicationResponse,
	E: ApplicationError + std::convert::From<crate::responses::BaseError> + std::convert::From<E>,
	crate::responses::BaseError: std::convert::From<E>,
{
	fn event_handler(&self) -> &'static TEventHandler<R, E>;
	async fn handle_event(&self, msg: Arc<dyn TEvent>) -> Result<(), E> {
		let context_manager = ContextManager::new();
		self._handle_event(msg, context_manager.clone()).await
	}
	async fn _handle_event(&self, msg: Arc<dyn TEvent>, context_manager: AtomicContextManager) -> Result<(), E> {
		// ! msg.topic() returns the name of event. It is crucial that it corresponds to the key registered on Event Handler.

		let handlers = self.event_handler().get(&msg.metadata().topic).ok_or_else(|| {
			eprintln!("Unprocessable Event Given! {:?}", msg);
			BaseError::NotFound
		})?;

		for handler in handlers.iter() {
			if let Err(err) = handler(msg.clone(), context_manager.clone()).await {
				// ! Safety:: BaseError Must Be Enforced To Be Accepted As Variant On ServiceError
				match err.into() {
					BaseError::StopSentinel => {
						eprintln!("Stop Sentinel Arrived!");

						break;
					}
					BaseError::StopSentinelWithEvent(event) => {
						eprintln!("Stop Sentinel With Event Arrived!");
						context_manager.write().await.push_back(event);
						break;
					}
					err => {
						eprintln!("Error Occurred While Handling Event! Error:{:?}", err);
					}
				}
			}
		}

		// Resursive case
		let incoming_event = context_manager.write().await.event_queue.pop_front();
		if let Some(event) = incoming_event {
			if let Err(err) = self._handle_event(event, context_manager.clone()).await {
				// ! Safety:: BaseError Must Be Enforced To Be Accepted As Variant On ServiceError
				eprintln!("{:?}", err);
			}
		}
		Ok(())
	}
}

#[async_trait]
pub trait TMessageBus<R, E, C>: TEventBus<R, E>
where
	responses::BaseError: std::convert::From<E>,
	R: ApplicationResponse,
	E: ApplicationError + std::convert::From<crate::responses::BaseError>,
	C: TCommand,
{
	fn command_handler(&self, context_manager: AtomicContextManager) -> impl TCommandService<R, E, C>;

	async fn handle(&self, message: C) -> Result<R, E> {
		let context_manager = ContextManager::new();
		let res = self.command_handler(context_manager.clone()).execute(message).await?;

		// Trigger event
		if !context_manager.read().await.event_queue.is_empty() {
			let event = context_manager.write().await.event_queue.pop_front();
			let _ = self._handle_event(event.unwrap(), context_manager.clone()).await;
		}
		Ok(res)
	}
}

/// init_event_handler creating macro
/// Not that crate must have `Dependency` struct with its own implementation
#[macro_export]
macro_rules! init_event_handler {
    (
		R: $response:ty,
		E: $error:ty $(,)?
        {
			$(
				$event:ty: [$($handler:expr $(=>($($injectable:ident $(( $($arg:ident),* ))? ),*))?),* $(,)? ]
			),*
			$(,)?
		}
    ) =>{
		pub fn event_handler() -> &'static ::ruva::prelude::TEventHandler<$response, $error>  {
			extern crate self as current_crate;
			static EVENT_HANDLER: ::std::sync::OnceLock<::ruva::prelude::TEventHandler<$response, $error>> = std::sync::OnceLock::new();
			EVENT_HANDLER.get_or_init(||{
			use current_crate::dependencies;
            let mut _map : ::ruva::prelude::TEventHandler<$response, $error> = ::ruva::prelude::HandlerMapper::new();
            $(
                _map.insert(
                    stringify!($event).into(),
                    vec![
                        $(
                            Box::new(
                                |e:std::sync::Arc<dyn TEvent>, context_manager: ::ruva::prelude::AtomicContextManager| -> std::pin::Pin<Box<dyn futures::Future<Output = Result<$response, $error>> + Send>>{


									#[allow(unused)]
									macro_rules! matcher{
										($a:ident)=>{
											context_manager.clone()
										}
									}

                                    Box::pin($handler(
                                        // * Convert event so event handler accepts not Arc<dyn TEvent> but `event_happend` type of message.
                                        // Safety:: client should access this vector of handlers by providing the corresponding event name
                                        // So, when it is followed, it logically doesn't make sense to cause an error.
                                        e.downcast_ref::<$event>().expect("Not Convertible!").clone(),
										$(
											// * Injectable functions are added here.
											$(dependencies::$injectable( $( $(matcher!($arg)),*)?),)*
										)?
                                    ))
                                }
                                ),
                        )*
                    ]
                );
            )*
            _map
        })
    }
};
	(
		E: $error:ty,
		R: $response:ty $(,)?
		{
			$(
				$event:ty: [$($handler:expr $(=>($($injectable:ident $(( $($arg:ident),* ))? ),*))?),* $(,)? ]
			),*
			$(,)?
		}
	) =>{
		init_event_handler!(
			R:$response,E:$error,
			{
				$(
					$event: [$($handler $(=>($($injectable $(( $($arg),* ))? ),*))?),* ]
				),*
			}
		)
	}
}
