const AbstractError = require('../../errors/AbstractError');

class ServiceAlreadyRunningError extends AbstractError {
  /**
   * @param {string} preset
   * @param {string} serviceName
   */
  constructor(preset, serviceName) {
    super(`Service ${serviceName} for ${preset} is already running. Please stop Docker Compose before`);

    this.preset = preset;
    this.serviceName = serviceName;
  }

  /**
   * Get preset
   *
   * @return {string}
   */
  getPreset() {
    return this.preset;
  }

  /**
   * Get service name
   *
   * @return {string}
   */
  getServiceName() {
    return this.serviceName;
  }
}

module.exports = ServiceAlreadyRunningError;
