class SPVError extends Error {
  constructor(message) {
    super(message);
    this.name = 'SPVError';
  }
}

module.exports = SPVError;
