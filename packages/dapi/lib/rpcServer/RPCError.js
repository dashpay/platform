class RPCError extends Error {
  constructor(code, message, data, originalStack) {
    super();

    this.code = code;
    this.message = message;
    this.data = data;

    if (originalStack) {
      this.stack = originalStack;
    }
  }
}

module.exports = RPCError;
