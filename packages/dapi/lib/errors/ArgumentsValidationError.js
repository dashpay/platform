class ArgumentsValidationError extends Error {
  constructor(message, originalStack, data) {
    super(message);
    if (originalStack) {
      this.stack = originalStack;
    }
    this.data = data;
  }
}

module.exports = ArgumentsValidationError;
