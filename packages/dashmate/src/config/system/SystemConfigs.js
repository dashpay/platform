class SystemConfigs {
  /**
   * @type {Object<string, function(): Config>}
   */
  #configGetters= {};

  /**
   * @param {Array<function(): Config>} configGetters
   */
  constructor(configGetters) {
    configGetters.forEach((getter) => {
      this.#configGetters[getter().getName()] = getter;
    });
  }

  /**
   * Get system config by name
   *
   * @param {string} name
   * @returns {Config}
   */
  get(name) {
    if (!this.has(name)) {
      throw new Error(`System config "${name}" does not exist`);
    }

    return this.#configGetters[name]();
  }

  /**
   * Get all system configs
   *
   * @returns {Config[]}
   */
  getAll() {
    return Object.values(this.#configGetters).map((getter) => getter());
  }

  /**
   * Check if system config exists
   *
   * @param {string} name
   * @returns {boolean}
   */
  has(name) {
    return Object.prototype.hasOwnProperty.call(this.#configGetters, name);
  }
}

module.exports = SystemConfigs;
