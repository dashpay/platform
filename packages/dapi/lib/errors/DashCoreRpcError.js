class DashCoreRpcError extends Error {
  constructor(message, originalStack, code) {
    super(message);
    if (originalStack) {
      this.stack = originalStack;
    }
    if (code) {
      this.code = code;
    }
  }
}

module.exports = DashCoreRpcError;
