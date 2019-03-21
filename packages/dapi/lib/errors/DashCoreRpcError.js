class DashCoreRpcError extends Error {
  constructor(message, originalStack) {
    super(message);
    if (originalStack) {
      this.stack = originalStack;
    }
  }
}

module.exports = DashCoreRpcError;
