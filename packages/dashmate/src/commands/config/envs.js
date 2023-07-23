const fs = require('fs');
const path = require('path');

const { Flags } = require('@oclif/core');

const ConfigBaseCommand = require('../../oclif/command/ConfigBaseCommand');
const generateEnvs = require('../../util/generateEnvs');

class ConfigEnvsCommand extends ConfigBaseCommand {
  /**
   * @param {Object} args
   * @param {Object} flags
   * @param {Config} config
   * @param {ConfigFile} configFile
   * @param {HomeDir} homeDir
   * @return {Promise<void>}
   */
  async runWithDependencies(
    args,
    {
      'output-file': outputFile,
    },
    config,
    configFile,
    homeDir,
  ) {
    let envOutput = '';

    for (const [key, value] of Object.entries(generateEnvs(configFile, config))) {
      envOutput += `${key}=${value}\n`;
    }

    envOutput += `DASHMATE_HOME_DIR=${homeDir.getPath()}\n`;

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
