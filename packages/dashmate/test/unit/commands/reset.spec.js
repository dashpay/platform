import { Listr } from 'listr2';
import Config from '../../../src/config/Config.js';
import ResetCommand from '../../../src/commands/reset.js';
import HomeDir from '../../../src/config/HomeDir.js';
import createPlatformSetupConfigs from '../../fixtures/platformSetupConfigs.js';
import renderNodeKeyBackup from '../../../src/config/renderNodeKeyBackup.js';

describe('Reset command', () => {
  let homeDir;
  let setUp;
  let stderr;
  let resetNodeTask;

  /**
   * @param {Config} config
   * @param {Object} flags
   */
  async function reset(config, flags) {
    await new ResetCommand().runWithDependencies(
      {},
      {
        verbose: false,
        hard: false,
        force: true,
        platform: false,
        'keep-data': false,
        ...flags,
      },
      config,
      resetNodeTask,
      homeDir,
    );
  }

  beforeEach(function beforeEach() {
    homeDir = HomeDir.createTemp();
    ({ setUp } = createPlatformSetupConfigs(homeDir));

    resetNodeTask = this.sinon.stub().callsFake(() => new Listr([]));

    stderr = '';
    this.sinon.stub(process.stderr, 'write').callsFake((chunk) => {
      stderr += chunk;

      return true;
    });
  });

  afterEach(() => {
    homeDir.remove();
  });

  describe('a hard Platform reset of a masternode', () => {
    // The registration names the node ID: setting Platform up again needs the
    // same key, and nothing of it is kept. A scripted reset uses --force and
    // never sees the confirmation, so this cannot wait for one.
    it('should name the node ID it deletes and how to back the key up, under --force too', async () => {
      const config = setUp('testnet', { masternode: true });

      await reset(config, { platform: true, hard: true, force: true });

      const { backupCommand, setupCommand } = renderNodeKeyBackup('testnet');

      expect(stderr).to.include(config.get('platform.drive.tenderdash.node.id'))
        .and.include(`  ${backupCommand}\n`)
        .and.include(`  ${setupCommand}\n`);
      expect(stderr).to.not.include(config.get('platform.drive.tenderdash.node.key'));
      expect(resetNodeTask).to.have.been.calledOnceWith(config);
    });

    // Platform-only setup serves the mainnet and testnet presets outside a
    // group, and refuses every other config: the key goes back where it was.
    [
      ['a member of the local group', 'local_1', 'local'],
      ['a config that is not a preset', 'evo-1', null],
    ].forEach(([description, name, group]) => {
      it(`should name the settings to restore for ${description}, not setup`, async () => {
        const config = new Config(name, setUp('testnet', { masternode: true }).getStoredOptions());
        config.set('group', group);

        await reset(config, { platform: true, hard: true, force: true });

        const { backupCommand } = renderNodeKeyBackup(name);

        expect(stderr).to.include(config.get('platform.drive.tenderdash.node.id'))
          .and.include(`  ${backupCommand}\n`)
          .and.include('platform.drive.tenderdash.node.key')
          .and.include('platform.drive.tenderdash.node.id')
          .and.include(homeDir.joinPath('config.json'))
          .and.include(`dashmate config render --config ${name}`);
        expect(stderr).to.not.include('dashmate setup');
        expect(stderr).to.not.include(config.get('platform.drive.tenderdash.node.key'));
      });
    });
  });

  it('should say nothing about a key that is not deleted or not registered', async () => {
    await reset(setUp('testnet', { masternode: true }), { platform: true, hard: false });
    await reset(setUp('testnet', { masternode: false }), { platform: true, hard: true });
    await reset(setUp('testnet', { masternode: true, evo: false }), { platform: true, hard: true });

    expect(stderr).to.not.include('node-key');
  });
});
