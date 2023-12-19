/**
 * @method trace
 * @method debug
 * @method info
 * @method warn
 * @method error
 * @method fatal
 * @method child
 */
class LoggerMock {
  /**
   * @param {SinonSandbox} sinon
   */
  constructor(sinon) {
    this.trace = sinon.stub();
    this.debug = sinon.stub();
    this.info = sinon.stub();
    this.warn = sinon.stub();
    this.error = sinon.stub();
    this.fatal = sinon.stub();
    this.child = () => this;
  }
}

module.exports = LoggerMock;
