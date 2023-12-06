//! Module containing the Put trait, allowing modification of objects (state transitions) on the Dash Platform.
use crate::platform::transition::TransitionContext;
use crate::{Error, Sdk};

use super::transition::TxId;

/// Trait implemented by objects that can be created or modified and pushed to the Dash Platform.
#[async_trait::async_trait]
pub trait Put {
    /// Put (create or update) object on the Platform.
    ///
    /// An asynchronous method provided by the Put trait that puts data on Dash Platform.
    /// It locks funds that will be used to pay for the operation, creates a state transition,
    /// signs it with appropriate keys, and broadcasts it to the platform.
    ///
    /// It returns a future that resolves to the saved object when the operation is successful,
    /// and the transaction is confirmed on the platform.
    ///
    /// ## Parameters
    ///
    /// - `sdk`: An instance of [Sdk].
    /// - `context` - contextual information about the transition that will be generated and broadcasted.
    /// It contains information about keys to use, payment details, etc.
    ///
    /// ## Timeouts
    ///
    /// Depending on network conditions, the operation may take a long time to complete.
    /// To prevent the operation from taking too long, consider using [Put::put_unconfirmed()].
    ///
    /// ## Canceling
    ///
    /// The returned future can be canceled by:
    ///
    /// * calling [`context.cancel()`](TransitionContext::cancel())
    /// * dropping the returned future
    ///
    /// ## Returns
    ///
    /// Returns:
    /// - ID of transaction on success
    /// - [`Err(Error)`](Error) when an error occurs
    async fn put(&self, sdk: &Sdk, context: Option<&TransitionContext>) -> Result<TxId, Error>;

    /// Put (create or update) object on the Platform, without waiting for confirmation.
    ///
    /// An asynchronous method provided by the Put trait that puts data on Dash Platform.
    /// It locks funds that will be used to pay for the operation, creates a state transition,
    /// signs it with appropriate keys, and broadcasts it to the platform.
    /// Unlike [Put::put()], it does not wait for the transaction to be confirmed.
    ///
    /// See [Put::put()](Put::put()) for more details.
    async fn put_and_wait(
        &self,
        sdk: &Sdk,
        context: Option<&TransitionContext>,
    ) -> Result<TxId, Error>;
}
