class Logger {
  constructor(writer) {
    this.writer = writer;
  }

  info(message, data) {
    this.writer.log(this.buildMessage(message, data));
  }

  error(message, data, error) {
    this.writer.error(this.buildMessage(message, data), error);
  }

  /**
   * @private
   */
  buildMessage(message, data = {}) {
    const date = (new Date()).toISOString();
    return `[${date}] ${message}\n${JSON.stringify(data, null, 2)}`;
  }
}

module.exports = Logger;
