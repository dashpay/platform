import fs from 'fs';
import path from 'path';
import {Flags} from "@oclif/core";
import {ConfigBaseCommand} from "../../oclif/command/ConfigBaseCommand.js";

export class ConfigEnvsCommand extends ConfigBaseCommand {
  static description = `Export config to envs

Export configuration options as Docker Compose envs
`;

  static flags = {
    ...ConfigBaseCommand.flags,
    'output-file': Flags.string({
      char: 'o',
      description: 'output to file',
      default: null,
    }),
  };

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

