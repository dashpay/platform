const AbstractError = require('../../errors/AbstractError');

class ContainerIsNotPresentError extends AbstractError {
  /**
   * @param {string} preset
   * @param {string} serviceName
   */
  constructor(preset, serviceName) {
    super(`Container ${serviceName} for ${preset} is not present`);

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

module.exports = ContainerIsNotPresentError;
