import { Parser } from '@oclif/core';
import ConfigDefaultCommand from '../../../../src/commands/config/default.js';
import ConfigFile from '../../../../src/config/configFile/ConfigFile.js';
import HomeDir from '../../../../src/config/HomeDir.js';
import getBaseConfigFactory from '../../../../configs/defaults/getBaseConfigFactory.js';

describe('Config default command', () => {
  const flags = {};

  let configFile;
  let baseConfigName;
  let consoleLog;
  let configFileRepository;

  /**
   * Parse the command line the same way oclif does at runtime, so the test
   * exercises the command's own argument definitions and not a hand-made object.
   *
   * @param {string[]} argv
   * @returns {Promise<Object>}
   */
  async function parseArgs(argv) {
    const { args } = await Parser.parse(argv, { args: ConfigDefaultCommand.args });

    return args;
  }

  beforeEach(async function beforeEach() {
    const getBaseConfig = getBaseConfigFactory(HomeDir.createTemp());
    const baseConfig = getBaseConfig();

    baseConfigName = baseConfig.getName();

    configFile = new ConfigFile([baseConfig], '1.0.0', null, baseConfigName, null);

    consoleLog = this.sinon.stub(console, 'log');

    // The command reads, changes and saves in one locked step, so the double
    // hands the mutation the config file the assertions look at
    configFileRepository = {
      update: this.sinon.stub().callsFake((mutate) => mutate(configFile)),
    };
  });

  it('should print default config name if config name is not specified', async function it() {
    const command = new ConfigDefaultCommand();

    const setDefaultConfigName = this.sinon.spy(configFile, 'setDefaultConfigName');

    await command.runWithDependencies(
      await parseArgs([]),
      flags,
      configFile,
      configFileRepository,
    );

    expect(consoleLog).to.be.calledOnceWith(baseConfigName);

    // Reading the default config name must not modify the config file
    expect(setDefaultConfigName).to.not.be.called();
    expect(configFileRepository.update).to.not.be.called();
    expect(configFile.getDefaultConfigName()).to.equal(baseConfigName);
  });

  it('should set specified config as default', async () => {
    const command = new ConfigDefaultCommand();

    configFile.setDefaultConfigName(null);

    await command.runWithDependencies(
      await parseArgs([baseConfigName]),
      flags,
      configFile,
      configFileRepository,
    );

    expect(configFile.getDefaultConfigName()).to.equal(baseConfigName);
    expect(consoleLog).to.be.calledOnceWith(`${baseConfigName} config set as default`);
  });
});
