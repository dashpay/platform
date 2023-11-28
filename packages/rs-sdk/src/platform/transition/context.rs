use tokio_util::sync::CancellationToken;

use crate::Sdk;

pub trait TransitionContext {
    /// Request cancellation of the transition.
    fn cancellation_token(&mut self) -> &mut CancellationToken;
}
pub struct TransitionContextImpl {
    cancellation_token: CancellationToken,
}

impl TransitionContextImpl {}

impl TransitionContext for TransitionContextImpl {
    fn cancellation_token(&mut self) -> &mut CancellationToken {
        &mut self.cancellation_token
    }
}

pub struct TransitionContextBuilder {}
impl TransitionContextBuilder {
    pub fn new(_sdk: &Sdk) -> Self {
        Self {}
    }

    pub fn build() -> impl TransitionContext {
        TransitionContextImpl {
            cancellation_token: CancellationToken::new(),
        }
    }
}
