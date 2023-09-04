const fs = require('fs');
const path = require('path');

const { Flags } = require('@oclif/core');

const ConfigBaseCommand = require('../../oclif/command/ConfigBaseCommand');

class ConfigEnvsCommand extends ConfigBaseCommand {
  /**
   * @param {Object} args
   * @param {Object} flags
   * @param {Config} config
   * @param {generateEnvs} generateEnvs
   * @return {Promise<void>}
   */
  async runWithDependencies(
    args,
    {
      'output-file': outputFile,
    },
    config,
    generateEnvs,
  ) {
    const envs = generateEnvs(config);

    let envOutput = '';

    for (const [key, value] of Object.entries(envs)) {
      envOutput += `${key}=${value}\n`;
    }

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
  'output-file': Flags.string({
    char: 'o',
    description: 'output to file',
    default: null,
  }),
};

module.exports = ConfigEnvsCommand;
