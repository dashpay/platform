const DashPlatformProtocol = require('@dashevo/dpp');

class IsolatedDashPlatformProtocol extends DashPlatformProtocol {
  /**
   * @param {Isolate} isolate
   * @param {Object} options
   * @param {StateRepository} options.stateRepository
   * @param {JsonSchemaValidator} options.jsonSchemaValidator
   */
  constructor(isolate, options) {
    super(options);

    this.isolate = isolate;
  }

  /**
   * Get Isolate
   *
   * @return {Isolate}
   */
  getIsolate() {
    return this.isolate;
  }

  /**
   * Dispose isolation
   */
  dispose() {
    this.getIsolate().dispose();
  }
}

module.exports = IsolatedDashPlatformProtocol;
