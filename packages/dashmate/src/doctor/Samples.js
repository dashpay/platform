export default class Samples {
  /**
   * @type {Date}
   */
  date;

  /**
   * @type {Object}
   */
  systemInfo = {};

  /**
   * @type {string}
   */
  #dockerError;

  /**
   * @type {string}
   */
  #dashmateVersion;

  /**
   * @type {Config}
   */
  #dashmateConfig;

  /**
   * @type {Object}
   */
  #services = {};

  /**
   * Whether each node on the host runs this dashmate. Host-wide, so it is kept apart from the
   * selected config's services.
   *
   * @type {Object}
   */
  #activation;

  constructor() {
    this.date = new Date();
  }

  setSystemInfo(systemInfo) {
    this.systemInfo = systemInfo;
  }

  getSystemInfo() {
    return this.systemInfo;
  }

  /**
   * @param {Error} error
   */
  setDockerError(error) {
    this.#dockerError = error.toString();
  }

  /**
   * @param {string} errorString
   */
  setStringifiedDockerError(errorString) {
    this.#dockerError = errorString;
  }

  /**
   * @return {string}
   */
  getStringifiedDockerError() {
    return this.#dockerError;
  }

  setDashmateVersion(version) {
    this.#dashmateVersion = version;
  }

  getDashmateVersion() {
    return this.#dashmateVersion;
  }

  /**
   * @param {Config} config
   */
  setDashmateConfig(config) {
    this.#dashmateConfig = config;
  }

  /**
   * @return {Config}
   */
  getDashmateConfig() {
    return this.#dashmateConfig;
  }

  setServiceInfo(service, key, data) {
    this.#services[service] = {
      ...(this.#services[service] ?? {}),
      [key]: data,
    };
  }

  getServices() {
    return this.#services;
  }

  getServiceInfo(service, key) {
    return this.#services[service]?.[key];
  }

  /**
   * @param {Object} activation
   */
  setActivation(activation) {
    this.#activation = activation;
  }

  /**
   * @return {Object|undefined}
   */
  getActivation() {
    return this.#activation;
  }
}
