const ConsensusError = require('./ConsensusError');

class UserNotFoundError extends ConsensusError {
  /**
   * @param {string} userId
   */
  constructor(userId) {
    super('User not found');

    this.userId = userId;
  }

  /**
   * Get user ID
   *
   * @return {string}
   */
  getUserId() {
    return this.userId;
  }
}

module.exports = UserNotFoundError;
