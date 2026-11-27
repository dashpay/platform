/**
 * @return {writeConfigTemplates}
 */
export default function writeConfigTemplatesFactory(renderServiceTemplates, writeServiceConfigs) {
  /**
   * @typedef {writeConfigTemplates}
   * @param {Config} config
   * @param {Object} [scope] - which service files to write, all by default
   *  (see renderServiceTemplates)
   */
  function writeConfigTemplates(config, scope = {}) {
    const serviceConfigs = renderServiceTemplates(config, scope);
    writeServiceConfigs(config.getName(), serviceConfigs);

    config.markAsSaved();
  }

  return writeConfigTemplates;
}
