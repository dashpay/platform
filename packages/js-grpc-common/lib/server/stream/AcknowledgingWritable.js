class AcknowledgingWritable {
  /**
   * @param {stream.Writable} writable
   */
  constructor(writable) {
    this.writable = writable;
  }

  /**
   * @param data
   * @return {Promise<any>}
   */
  write(data) {
    return new Promise((resolve, reject) => {
      const callback = (error) => {
        if (error) {
          return reject(error);
        }
        return resolve(true);
      };
      const handler = this.attachHandler(callback);
      this.writable.write(data, handler);
    });
  }

  /**
   * @private
   * @param callback
   * @return {handler}
   */
  createHandler(callback) {
    const handler = (error) => {
      this.detachHandler(handler);
      callback(error);
    };
    return handler;
  }

  /**
   * @private
   * @param {function} handler
   */
  detachHandler(handler) {
    this.writable.off('error', handler);
    this.writable.off('drain', handler);
  }

  /**
   * @private
   * @param {function} callback
   * @return {handler}
   */
  attachHandler(callback) {
    const handler = this.createHandler(callback);
    this.writable.once('error', handler);
    this.writable.once('drain', handler);
    return handler;
  }
}

module.exports = AcknowledgingWritable;
