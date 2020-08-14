const AbstractError = require('../../errors/AbstractError');

class ContainerIsNotPresentError extends AbstractError {
  /**
   * @param {string} serviceName
   */
  constructor(serviceName) {
    super(`Container ${serviceName} is not present`);

    this.serviceName = serviceName;
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
