const $RefParser = require('@apidevtools/json-schema-ref-parser');
const ConfigBaseCommand = require('../../oclif/command/ConfigBaseCommand');
const configJsonSchema = require('../../../configs/schema/configJsonSchema');
const getPropertyDefinitionByPath = require('../../util/getPropertyDefinitionByPath');

class ConfigSetCommand extends ConfigBaseCommand {
  /**
   * @param args
   * @param flags
   * @param {Config} config
   * @return {Promise<void>}
   */
  async runWithDependencies(
    {
      option: optionPath,
      value: optionValue,
    },
    flags,
    config,
  ) {
    if (optionValue === 'null') {
      // eslint-disable-next-line no-param-reassign
      optionValue = null;
    }

    // check for existence
    config.get(optionPath);

    const configSchema = await $RefParser.dereference(configJsonSchema);
    const schema = getPropertyDefinitionByPath(configSchema, optionPath);

    if (!schema) {
      throw new Error(`Could not find schema for option path ${optionPath}`);
    }

    let schemaType = schema.type;

    if (Array.isArray(schemaType)) {
      [schemaType] = schemaType;
    }

    if (schemaType === 'object' || schemaType === 'array') {
      config.set(optionPath, JSON.parse(optionValue));
    } else {
      config.set(optionPath, optionValue);
    }

    // eslint-disable-next-line no-console
    console.log(`${optionPath} set to ${JSON.stringify(config.get(optionPath))}`);
  }
}

ConfigSetCommand.description = `Set config option

Sets a configuration option in the default config
`;

ConfigSetCommand.args = [{
  name: 'option',
  required: true,
  description: 'option path',
}, {
  name: 'value',
  required: true,
  description: 'the option value',
}];

ConfigSetCommand.flags = {
  ...ConfigBaseCommand.flags,
};

module.exports = ConfigSetCommand;
