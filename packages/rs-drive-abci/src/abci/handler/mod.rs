// MIT LICENSE
//
// Copyright (c) 2021 Dash Core Group
//
// Permission is hereby granted, free of charge, to any
// person obtaining a copy of this software and associated
// documentation files (the "Software"), to deal in the
// Software without restriction, including without
// limitation the rights to use, copy, modify, merge,
// publish, distribute, sublicense, and/or sell copies of
// the Software, and to permit persons to whom the Software
// is furnished to do so, subject to the following
// conditions:
//
// The above copyright notice and this permission notice
// shall be included in all copies or substantial portions
// of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
// ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
// TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
// PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
// SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
// CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
// OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
// IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
// DEALINGS IN THE SOFTWARE.
//

//! Tenderdash ABCI Handlers.
//!
//! This module defines the `TenderdashAbci` trait and implements it for type `Platform`.
//!
//! Handlers in this function MUST be version agnostic, meaning that for all future versions, we
//! can only make changes that are backwards compatible. Otherwise new calls must be made instead.
//!

mod add_state_transition_subscription;
mod check_tx;
mod echo;
pub mod error;
mod extend_vote;
mod finalize_block;
mod info;
mod init_chain;
mod prepare_proposal;
mod process_proposal;
mod verify_vote_extension;

#[allow(unused_imports)]
pub use add_state_transition_subscription::add_state_transition_subscription;
pub use check_tx::check_tx;
pub use echo::echo;
pub use extend_vote::extend_vote;
pub use finalize_block::finalize_block;
pub use info::info;
pub use init_chain::init_chain;
pub use prepare_proposal::prepare_proposal;
pub use process_proposal::process_proposal;
pub use verify_vote_extension::verify_vote_extension;
