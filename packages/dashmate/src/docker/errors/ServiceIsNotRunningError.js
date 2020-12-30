const AbstractError = require('../../errors/AbstractError');

class ServiceIsNotRunningError extends AbstractError {
  /**
   * @param {string} configName
   * @param {string} serviceName
   */
  constructor(configName, serviceName) {
    super(`Service ${serviceName} for ${configName} is not running. Please run the service first.`);

    this.configName = configName;
    this.serviceName = serviceName;
  }

  /**
   * Get config name
   *
   * @return {string}
   */
  getConfigName() {
    return this.configName;
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

module.exports = ServiceIsNotRunningError;
