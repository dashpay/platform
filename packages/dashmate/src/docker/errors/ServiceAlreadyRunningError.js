const AbstractError = require('../../errors/AbstractError');

class ServiceAlreadyRunningError extends AbstractError {
  /**
   * @param {string} serviceName
   */
  constructor(serviceName) {
    super(`Service ${serviceName} is already running. Please stop Docker Compose before`);

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

module.exports = ServiceAlreadyRunningError;
