import AbstractError from '../../errors/AbstractError.js';

export default class ServiceAlreadyRunningError extends AbstractError {
  /**
   * @param {string} serviceName
   */
  constructor(serviceName) {
    super(`Service ${serviceName} is already running. Please stop it before`);

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
