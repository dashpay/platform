const fs = require('fs');
const path = require('path');

const { flags: flagTypes } = require('@oclif/command');

const { HOME_DIR_PATH } = require('../../constants');

const ConfigBaseCommand = require('../../oclif/command/ConfigBaseCommand');

class ConfigEnvsCommand extends ConfigBaseCommand {
  /**
   * @param {Object} args
   * @param {Object} flags
   * @param {Config} config
   * @return {Promise<void>}
   */
  async runWithDependencies(
    args,
    {
      'output-file': outputFile,
    },
    config,
  ) {
    let envOutput = '';

    for (const [key, value] of Object.entries(config.toEnvs())) {
      envOutput += `${key}=${value}\n`;
    }

    envOutput += `DASHMATE_HOME_DIR=${HOME_DIR_PATH}\n`;

    if (outputFile !== null) {
      const outputFilePath = path.resolve(process.cwd(), outputFile);

      fs.writeFileSync(outputFilePath, envOutput, 'utf8');
    } else {
      // eslint-disable-next-line no-console
      console.log(envOutput);
    }
  }
}

ConfigEnvsCommand.description = `Export config to envs

Export configuration options as Docker Compose envs
`;

ConfigEnvsCommand.flags = {
  ...ConfigBaseCommand.flags,
  'output-file': flagTypes.string({
    char: 'o',
    description: 'output to file',
    default: null,
  }),
};

module.exports = ConfigEnvsCommand;
