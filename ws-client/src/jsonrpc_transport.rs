//! JSONRPC WebSocket Transport module.
//!
//! Wraps the underlying WebSocket transport with specific JSONRPC details.

use crate::{
	manager::RequestManager,
	transport::{self, WsConnectError},
};
use jsonrpsee_types::{
	client::{BatchMessage, NotificationMessage, RequestMessage, SubscriptionMessage},
	error::Error,
	v2::dummy::{JsonRpcCall, JsonRpcNotification, JsonRpcRequest, JsonRpcResponse},
};
use serde::de::DeserializeOwned;

/// JSONRPC WebSocket sender.
#[derive(Debug)]
pub struct Sender {
	transport: transport::Sender,
}

impl Sender {
	/// Creates a new JSONRPC sender.
	pub fn new(transport: transport::Sender) -> Self {
		Self { transport }
	}

	/// Send a batch request.
	pub async fn start_batch_request(
		&mut self,
		batch: BatchMessage,
		request_manager: &mut RequestManager,
	) -> Result<(), Error> {
		todo!();
		/*let req_id = request_manager.next_request_id()?;
		let mut calls = Vec::with_capacity(batch.requests.len());
		let mut ids = Vec::with_capacity(batch.requests.len());

		for (method, params) in batch.requests {
			let batch_id = request_manager.next_batch_id();
			ids.push(batch_id);
			calls.push(jsonrpc::Call::MethodCall(jsonrpc::MethodCall {
				jsonrpc: jsonrpc::Version::V2,
				method,
				params,
				id: jsonrpc::Id::Num(batch_id),
			}));
		}

		if let Err(send_back) = request_manager.insert_pending_batch(ids, batch.send_back, req_id) {
			request_manager.reclaim_request_id(req_id);
			let _ = send_back.send(Err(Error::InvalidRequestId));
			return Err(Error::InvalidRequestId);
		};

		let res =
			self.transport.send_request(Request::Batch(calls)).await.map_err(|e| Error::TransportError(Box::new(e)));

		match res {
			Ok(_) => Ok(()),
			Err(e) => {
				request_manager.reclaim_request_id(req_id);
				Err(e)
			}
		}*/
	}

	/// Sends a request to the server but it doesn’t wait for a response.
	/// Instead, you have keep the request ID and use the Receiver to get the response.
	///
	/// Returns Ok() if the request was successfully sent otherwise Err(_).
	pub async fn start_request(
		&mut self,
		request: RequestMessage,
		request_manager: &mut RequestManager,
	) -> Result<(), Error> {
		let id = match request_manager.next_request_id() {
			Ok(id) => id,
			Err(err) => {
				let str_err = err.to_string();
				request.send_back.map(|tx| tx.send(Err(err)));
				return Err(Error::Custom(str_err));
			}
		};
		let req = JsonRpcCall::new(id, request.method.inner(), request.params.inner());
		match self.transport.send_request(req).await {
			Ok(_) => {
				request_manager.insert_pending_call(id, request.send_back).expect("ID unused checked above; qed");
				Ok(())
			}
			Err(e) => {
				let str_err = e.to_string();
				let _ = request.send_back.map(|tx| tx.send(Err(Error::TransportError(Box::new(e)))));
				Err(Error::Custom(str_err))
			}
		}
	}

	/// Sends a notification to the server. The notification doesn't need any response.
	///
	/// Returns `Ok(())` if the notification was successfully sent otherwise `Err(_)`.
	pub async fn send_notification<'a>(&mut self, notif: NotificationMessage) -> Result<(), Error> {
		let notif = JsonRpcNotification::new(notif.method.inner(), notif.params.inner());
		self.transport.send_request(notif).await.map_err(|e| Error::TransportError(Box::new(e)))
	}

	/// Sends a request to the server to start a new subscription but it doesn't wait for a response.
	/// Instead, you have keep the request ID and use the [`Receiver`] to get the response.
	///
	/// Returns `Ok()` if the request was successfully sent otherwise `Err(_)`.
	pub async fn start_subscription(
		&mut self,
		sub: SubscriptionMessage,
		request_manager: &mut RequestManager,
	) -> Result<(), Error> {
		let id = match request_manager.next_request_id() {
			Ok(id) => id,
			Err(err) => {
				let str_err = err.to_string();
				let _ = sub.send_back.send(Err(err));
				return Err(Error::Custom(str_err));
			}
		};
		let req = JsonRpcCall::new(id, sub.subscribe_method.inner(), sub.params.inner());

		if let Err(e) = self.transport.send_request(req).await {
			let str_err = e.to_string();
			let _ = sub.send_back.send(Err(Error::TransportError(Box::new(e))));
			return Err(Error::Custom(str_err));
		}
		request_manager
			.insert_pending_subscription(id, sub.send_back, sub.unsubscribe_method)
			.expect("Request ID unused checked above; qed");
		Ok(())
	}
}

/// JSONRPC WebSocket receiver.
#[derive(Debug)]
pub struct Receiver {
	transport: transport::Receiver,
}

impl Receiver {
	/// Create a new JSONRPC WebSocket receiver.
	pub fn new(transport: transport::Receiver) -> Self {
		Self { transport }
	}

	/// Reads the next response, fails if the response ID was not a number.
	pub async fn next_response<T>(&mut self) -> Result<JsonRpcResponse<T>, WsConnectError>
	where
		T: DeserializeOwned + std::fmt::Debug,
	{
		self.transport.next_response().await
	}
}
