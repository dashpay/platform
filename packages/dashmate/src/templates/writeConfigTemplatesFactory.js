/**
 * @return {writeConfigTemplates}
 */
export default function writeConfigTemplatesFactory(renderServiceTemplates, writeServiceConfigs) {
  /**
   * @typedef {writeConfigTemplates}
   * @param {Config} config
   */
  function writeConfigTemplates(config) {
    const serviceConfigs = renderServiceTemplates(config);
    writeServiceConfigs(config.getName(), serviceConfigs);

    config.markAsSaved();
  }

  return writeConfigTemplates;
}
