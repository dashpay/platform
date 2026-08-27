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
  /**
   * The config dashmate acts on when a command names none.
   *
   * Collected so a report can render the same commands the node would: a
   * command that names the default config says nothing an operator can act on,
   * and one that omits a non-default config acts on the wrong node.
   *
   * @param {string|null} name
   */
  setDefaultConfigName(name) {
    this.defaultConfigName = name;
  }

  /**
   * @return {string|null}
   */
  getDefaultConfigName() {
    return this.defaultConfigName ?? null;
  }

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
}
