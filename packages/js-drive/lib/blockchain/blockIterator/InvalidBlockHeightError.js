class InvalidBlockHeightError extends Error {
  constructor(blockHeight) {
    super();

    this.name = this.constructor.name;
    this.message = `Block height ${blockHeight} out of bounds`;
    this.blockHeight = blockHeight;

    Error.captureStackTrace(this, this.constructor);
  }
}

module.exports = InvalidBlockHeightError;
