const fs = require('fs');
const path = require('path');

const { flags: flagTypes } = require('@oclif/command');

const ConfigBaseCommand = require('../../oclif/command/ConfigBaseCommand');

class ConfigEnvsCommand extends ConfigBaseCommand {
  /**
   * @param {Object} args
   * @param {Object} flags
   * @param {Config} config
   * @param {string} homeDirPath
   * @return {Promise<void>}
   */
  async runWithDependencies(
    args,
    {
      'output-file': outputFile,
    },
    config,
    homeDirPath,
  ) {
    let envOutput = '';

    for (const [key, value] of Object.entries(config.toEnvs())) {
      envOutput += `${key}=${value}\n`;
    }

    envOutput += `MN_HOME_DIR=${homeDirPath}\n`;

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
