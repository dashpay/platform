class ReplicaSetInitError extends Error {
  /**
   * @param {string} message
   * @param {Object} replicaStatus
   */
  constructor(message, replicaStatus) {
    super();

    this.name = this.constructor.name;
    this.message = message;
    this.replicaStatus = replicaStatus;

    Error.captureStackTrace(this, this.constructor);
  }

  /**
   * Get replica set status
   *
   * @returns {Object}
   */
  getReplicaStatus() {
    return this.replicaStatus;
  }
}

module.exports = ReplicaSetInitError;
