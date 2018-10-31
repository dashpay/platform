class Logger {
  constructor(writer) {
    this.writer = writer;
  }

  info(message, data) {
    this.writer.log(this.buildMessage(message, data));
  }

  error(message, data) {
    this.writer.error(this.buildMessage(message, data));
  }

  /**
   * @private
   */
  // eslint-disable-next-line class-methods-use-this
  buildMessage(message, data = {}) {
    const date = (new Date()).toISOString();
    return `[${date}] ${message}\n${JSON.stringify(data, null, 2)}`;
  }
}

module.exports = Logger;
