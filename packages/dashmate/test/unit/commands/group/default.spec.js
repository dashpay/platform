import { Parser } from '@oclif/core';
import GroupDefaultCommand from '../../../../src/commands/group/default.js';
import ConfigFile from '../../../../src/config/configFile/ConfigFile.js';
import HomeDir from '../../../../src/config/HomeDir.js';
import getBaseConfigFactory from '../../../../configs/defaults/getBaseConfigFactory.js';

describe('Group default command', () => {
  const flags = {};
  const groupName = 'local';

  let consoleLog;
  let configFile;
  let configFileRepository;

  /**
   * Parse the command line the same way oclif does at runtime, so the test
   * exercises the command's own argument definitions and not a hand-made object.
   *
   * @param {string[]} argv
   * @returns {Promise<Object>}
   */
  async function parseArgs(argv) {
    const { args } = await Parser.parse(argv, { args: GroupDefaultCommand.args });

    return args;
  }

  /**
   * @param {string|null} defaultGroupName
   * @returns {ConfigFile}
   */
  function createConfigFile(defaultGroupName) {
    const getBaseConfig = getBaseConfigFactory(HomeDir.createTemp());
    const baseConfig = getBaseConfig();

    baseConfig.set('group', groupName);

    configFile = new ConfigFile([baseConfig], '1.0.0', null, null, defaultGroupName);

    return configFile;
  }

  beforeEach(function beforeEach() {
    consoleLog = this.sinon.stub(console, 'log');

    // The command reads, changes and saves in one locked step, so the double
    // hands the mutation the config file the assertions look at
    configFileRepository = {
      update: this.sinon.stub().callsFake((mutate) => mutate(configFile)),
    };
  });

  it('should print default group name if group name is not specified', async function it() {
    createConfigFile(groupName);

    const command = new GroupDefaultCommand();

    const setDefaultGroupName = this.sinon.spy(configFile, 'setDefaultGroupName');

    await command.runWithDependencies(
      await parseArgs([]),
      flags,
      configFile,
      configFileRepository,
    );

    expect(consoleLog).to.be.calledOnceWith(groupName);

    // Reading the default group name must not modify the config file
    expect(setDefaultGroupName).to.not.be.called();
    expect(configFileRepository.update).to.not.be.called();
    expect(configFile.getDefaultGroupName()).to.equal(groupName);
  });

  it('should set specified group as default', async () => {
    createConfigFile(null);

    const command = new GroupDefaultCommand();

    await command.runWithDependencies(
      await parseArgs([groupName]),
      flags,
      configFile,
      configFileRepository,
    );

    expect(configFile.getDefaultGroupName()).to.equal(groupName);
    expect(consoleLog).to.be.calledOnceWith(`${groupName} group set as default`);
  });
});
