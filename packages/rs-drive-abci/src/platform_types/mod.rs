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

/// The outcome of a block execution
pub mod block_execution_outcome;
/// The block proposal
pub mod block_proposal;
/// A clean version of the the requst to finalize a block
pub mod cleaned_abci_messages;
/// The commit
pub mod commit;
/// Epoch
pub mod epoch_info;
/// The execution event result
pub mod event_execution_result;
/// Masternode
pub mod masternode;
/// Main platform structs, not versioned
pub mod platform;
/// Platform state
pub mod platform_state;
/// Required identity public key set for system identities
pub mod required_identity_public_key_set;
/// Signature verification quorums
pub mod signature_verification_quorums;
/// The state transition execution result as part of the block execution outcome
pub mod state_transitions_processing_result;
/// System identity public keys
pub mod system_identity_public_keys;
/// The validator module
/// A validator is a masternode that can participate in consensus by being part of a validator set
pub mod validator;
/// Quorum methods
pub mod validator_set;
/// Verify chain lock result
pub mod verify_chain_lock_result;
/// Withdrawal types
pub mod withdrawal;
