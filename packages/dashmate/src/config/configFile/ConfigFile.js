import { Config } from '../Config.js';
import { ConfigIsNotPresentError } from '../errors/ConfigIsNotPresentError.js';
import { GroupIsNotPresentError } from '../errors/GroupIsNotPresentError.js';
import { ConfigAlreadyPresentError } from '../errors/ConfigAlreadyPresentError.js';

export class ConfigFile {
  /**
   * @param {Config[]} configs
   * @param {string} configFormatVersion
   * @param {string|null} projectId
   * @param {string|null} defaultConfigName
   * @param {string|null} defaultGroupName
   */
  constructor(
    configs,
    configFormatVersion,
    projectId,
    defaultConfigName,
    defaultGroupName,
  ) {
    this.setConfigs(configs);
    this.configFormatVersion = configFormatVersion;
    this.defaultConfigName = defaultConfigName;
    this.defaultGroupName = defaultGroupName;
    this.projectId = projectId || null;
    this.changed = false;
  }

  /**
   * Get call configs
   *
   * @returns {Config[]}
   */
  getAllConfigs() {
    return Object.values(this.configsMap);
  }

  /**
   * Set current config name
   *
   * @param {string|null} name
   * @returns {ConfigFile}
   */
  setDefaultConfigName(name) {
    if (name !== null && !this.isConfigExists(name)) {
      throw new ConfigIsNotPresentError(name);
    }

    this.defaultConfigName = name;

    this.changed = true;

    return this;
  }

  /**
   * Get current config name if set
   *
   * @returns {string|null}
   */
  getDefaultConfigName() {
    return this.defaultConfigName;
  }

  /**
   * Get current config if set
   *
   * @returns {Config|null}
   */
  getDefaultConfig() {
    if (this.getDefaultConfigName() === null) {
      return null;
    }

    return this.getConfig(
      this.getDefaultConfigName(),
    );
  }

  /**
   * Set current config format version
   *
   * @param {string} version
   * @returns {ConfigFile}
   */
  setConfigFormatVersion(version) {
    this.configFormatVersion = version;

    return this;
  }

  /**
   * Get current config format version if set
   *
   * @returns {string|null}
   */
  getConfigFormatVersion() {
    return this.configFormatVersion;
  }

  /**
   * Check is group exists
   *
   * @param {string} name
   * @return {boolean}
   */
  isGroupExists(name) {
    return Object.entries(this.configsMap)
      .filter(([, config]) => config.get('group') === name).length !== 0;
  }

  /**
   * Set default group name
   *
   * @param {string} defaultGroupName
   */
  setDefaultGroupName(defaultGroupName) {
    if (!this.isGroupExists(defaultGroupName)) {
      throw new GroupIsNotPresentError(defaultGroupName);
    }

    this.changed = true;

    this.defaultGroupName = defaultGroupName;
  }

  /**
   * Get default group name
   *
   * @return {string}
   */
  getDefaultGroupName() {
    return this.defaultGroupName;
  }

  /**
   * Get group configs
   *
   * @param {string} name
   * @return {Config[]}
   */
  getGroupConfigs(name) {
    if (!this.isGroupExists(name)) {
      throw new GroupIsNotPresentError(name);
    }

    return Object.entries(this.configsMap)
      .filter(([, config]) => config.get('group') === name)
      .map(([, config]) => config);
  }

  /**
   * Get config by name
   *
   * @param {string} name
   */
  getConfig(name) {
    if (!this.isConfigExists(name)) {
      throw new ConfigIsNotPresentError(name);
    }

    return this.configsMap[name];
  }

  /**
   * Is config exists
   *
   * @param {string} name
   * @returns {boolean}
   */
  isConfigExists(name) {
    return Object.prototype.hasOwnProperty.call(this.configsMap, name);
  }

  /**
   * Create a new config
   *
   * @param {string} name
   * @param {string} fromConfigName - Set options from another config
   * @returns {ConfigFile}
   */
  createConfig(name, fromConfigName) {
    if (this.isConfigExists(name)) {
      throw new ConfigAlreadyPresentError(name);
    }

    const fromConfig = this.getConfig(fromConfigName);

    this.configsMap[name] = new Config(name, fromConfig.getOptions());
    this.configsMap[name].markAsChanged();

    this.changed = true;

    return this.configsMap[name];
  }

  /**
   * Remove config by name
   *
   * @param {string} name
   * @returns {ConfigFile}
   */
  removeConfig(name) {
    if (!this.isConfigExists(name)) {
      throw new ConfigIsNotPresentError(name);
    }

    if (this.getDefaultConfigName() === name) {
      this.setDefaultConfigName(null);
    }

    delete this.configsMap[name];

    this.changed = true;

    return this;
  }

  /**
   * Set config
   *
   * @param {Config} config
   */
  setConfig(config) {
    this.configsMap[config.getName()] = config;

    this.changed = true;
  }

  /**
   * Set configs
   * @param {Config[]} configs
   */
  setConfigs(configs) {
    this.configsMap = configs.reduce((configsMap, config) => {
      // eslint-disable-next-line no-param-reassign
      configsMap[config.getName()] = config;

      return configsMap;
    }, {});

    this.changed = true;
  }

  /**
   * Set project id
   *
   * @param {string} projectId
   * @returns {ConfigFile}
   */
  setProjectId(projectId) {
    this.projectId = projectId;

    this.changed = true;

    return this;
  }

  /**
   * Get project id
   *
   * @returns {string}
   */
  getProjectId() {
    return this.projectId;
  }

  /**
   * Is config file changed
   *
   * @return {boolean}
   */
  isChanged() {
    return this.changed
      || Object.values(this.configsMap)
        .find((config) => config.isChanged()) !== undefined;
  }

  /**
   * Mark config file as changed
   */
  markAsChanged() {
    this.changed = true;
  }

  /**
   * Mark config file as saved
   */
  markAsSaved() {
    this.changed = false;
  }

  /**
   * Get config file as plain object
   *
   * @return {{
   *  configs: Config,
   *  defaultGroupName: string,
   *  configFormatVersion: (string|null),
   *  defaultConfigName: (string|null),
   *  [projectId]: (string|null),
   * }}
   */
  toObject() {
    const result = {
      configFormatVersion: this.getConfigFormatVersion(),
      defaultConfigName: this.getDefaultConfigName(),
      defaultGroupName: this.getDefaultGroupName(),
      configs: this.getAllConfigs().reduce((configsMap, config) => {
        // eslint-disable-next-line no-param-reassign
        configsMap[config.getName()] = config.getOptions();

        return configsMap;
      }, {}),
    };

    const projectId = this.getProjectId();
    if (projectId !== null) {
      result.projectId = projectId;
    }

    return result;
  }
}
