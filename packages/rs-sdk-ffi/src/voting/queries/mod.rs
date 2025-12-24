// Voting queries
pub mod vote_polls_by_end_date;

// Re-export all public functions for convenient access
pub use vote_polls_by_end_date::dash_sdk_voting_get_vote_polls_by_end_date;
