export default class Samples {
  date;

  systemInfo = {};

  #dashmateVersion = null;

  #dashmateConfig = null;

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

  setDashmateVersion(version) {
    this.#dashmateVersion = version;
  }

  getDashmateVersion() {
    return this.#dashmateVersion;
  }

  setDashmateConfig(config) {
    this.#dashmateConfig = config;
  }

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

  getServiceInfo(service) {
    return this.#services[service];
  }

  getServiceInfo(service, key) {
    return this.#services[service]?.[key];
  }
}
