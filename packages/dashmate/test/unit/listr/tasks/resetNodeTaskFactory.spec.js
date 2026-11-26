import fs from 'fs';
import path from 'path';
import { Listr } from 'listr2';
import HomeDir from '../../../../src/config/HomeDir.js';
import DefaultConfigs from '../../../../src/config/DefaultConfigs.js';
import Config from '../../../../src/config/Config.js';
import ConfigFile from '../../../../src/config/configFile/ConfigFile.js';
import getBaseConfigFactory from '../../../../configs/defaults/getBaseConfigFactory.js';
import getTestnetConfigFactory from '../../../../configs/defaults/getTestnetConfigFactory.js';
import getLocalConfigFactory from '../../../../configs/defaults/getLocalConfigFactory.js';
import resetNodeTaskFactory from '../../../../src/listr/tasks/resetNodeTaskFactory.js';
import generateTenderdashNodeKey from '../../../../src/tenderdash/generateTenderdashNodeKey.js';
import deriveTenderdashNodeId from '../../../../src/tenderdash/deriveTenderdashNodeId.js';

describe('resetNodeTaskFactory', () => {
  let homeDir;
  let getBaseConfig;
  let getTestnetConfig;
  let getLocalConfig;
  let config;
  let configFile;
  let dockerCompose;
  let resetNodeTask;

  /**
   * Make the given configs the config file's contents and reset the first one
   *
   * @param {Config[]} configs
   */
  function useConfigs(configs) {
    [config] = configs;

    configFile = new ConfigFile([getBaseConfig(), ...configs], '4.1.0', 'abcdef12', config.getName(), null);

    resetNodeTask = resetNodeTaskFactory(
      dockerCompose,
      {},
      null,
      null,
      new DefaultConfigs([getBaseConfig, getTestnetConfig, getLocalConfig]),
      configFile,
      homeDir,
      () => ({}),
      () => [],
    );
  }

  /**
   * @param {...string} segments
   * @return {string}
   */
  function writeFile(...segments) {
    const filePath = homeDir.joinPath(...segments);
    fs.mkdirSync(path.dirname(filePath), { recursive: true });
    fs.writeFileSync(filePath, 'content');

    return filePath;
  }

  beforeEach(function beforeEach() {
    homeDir = HomeDir.createTemp();

    getBaseConfig = getBaseConfigFactory(homeDir);
    getTestnetConfig = getTestnetConfigFactory(homeDir, getBaseConfig);
    getLocalConfig = getLocalConfigFactory(getBaseConfig);

    dockerCompose = {
      isNodeRunning: this.sinon.stub().resolves(false),
      down: this.sinon.stub().resolves(),
      rm: this.sinon.stub().resolves(),
      getVolumeNames: this.sinon.stub().resolves([]),
    };

    // A testnet Evo fullnode: it runs platform, but no identity is registered
    // on chain for it, so dashmate is the only source of its node key.
    const testnetConfig = getTestnetConfig();
    testnetConfig.set('core.masternode.enable', false);
    testnetConfig.set('platform.enable', true);
    const key = generateTenderdashNodeKey();
    testnetConfig.set('platform.drive.tenderdash.node.key', key);
    testnetConfig.set('platform.drive.tenderdash.node.id', deriveTenderdashNodeId(key));

    useConfigs([testnetConfig]);
  });

  afterEach(() => {
    homeDir.remove();
  });

  function reset(ctx) {
    return new Listr([{ task: () => resetNodeTask(config) }], { renderer: 'silent' })
      .run({
        isHardReset: false,
        isForce: true,
        keepData: true,
        removeConfig: false,
        ...ctx,
      });
  }

  function hardReset(ctx) {
    return reset({ isHardReset: true, ...ctx });
  }

  // Resetting platform options clears the node identity while the config stays
  // a platform fullnode. The command saves the config before rendering, so a key
  // first generated at render time would never reach config.json and the node
  // ID would change on every render.
  it('should leave a fullnode with a new identity after a platform reset', async () => {
    const { key: previousKey } = config.get('platform.drive.tenderdash.node');

    await hardReset({ isPlatformOnlyReset: true });

    const { key, id } = config.get('platform.drive.tenderdash.node');
    expect(key).to.be.a('string').and.not.equal(previousKey);
    expect(id).to.equal(deriveTenderdashNodeId(key));
  });

  // An evonode must run the identity its operator registered on chain. A key
  // generated here would give it an unregistered node ID.
  it('should not generate an identity when platform is reset on a masternode', async () => {
    config.set('core.masternode.enable', true);

    await hardReset({ isPlatformOnlyReset: true });

    expect(config.get('core.masternode.enable')).to.be.true();
    expect(config.get('platform.drive.tenderdash.node')).to.deep.equal({ id: null, key: null });
  });

  // A full reset restores the testnet preset, which is a masternode: its
  // identity must come from the operator, never from dashmate.
  it('should not generate an identity when a full reset restores a masternode', async () => {
    await hardReset({ isPlatformOnlyReset: false });

    expect(config.get('core.masternode.enable')).to.be.true();
    expect(config.get('platform.drive.tenderdash.node')).to.deep.equal({ id: null, key: null });
  });

  describe('a config without defaults', () => {
    let corePath;
    let platformPath;

    /**
     * @param {string} name
     * @param {string|null} [group]
     */
    function useConfigWithoutDefaults(name, group = null) {
      const options = getTestnetConfig().getStoredOptions();
      options.group = group;

      useConfigs([new Config(name, options)]);

      corePath = writeFile(name, 'core', 'dash.conf');
      platformPath = writeFile(name, 'platform', 'drive', 'tenderdash', 'config.toml');
    }

    // dashmate knows Platform defaults only for its presets. Without them a hard
    // Platform reset has nothing to restore, and removing the whole config
    // instead would take Core's settings with it.
    it('should refuse a hard Platform reset before touching any service', async () => {
      useConfigWithoutDefaults('devnet-ops');

      await expect(hardReset({ isPlatformOnlyReset: true, isForce: false }))
        .to.be.rejectedWith(/'devnet-ops'[\s\S]*dashmate reset --config devnet-ops --platform[\s\S]*dashmate reset --config devnet-ops --hard/);

      expect(dockerCompose.isNodeRunning).to.not.have.been.called();
      expect(dockerCompose.rm).to.not.have.been.called();
      expect(dockerCompose.down).to.not.have.been.called();
      expect(dockerCompose.getVolumeNames).to.not.have.been.called();
      expect(configFile.isConfigExists('devnet-ops')).to.be.true();
      expect(fs.existsSync(corePath)).to.be.true();
      expect(fs.existsSync(platformPath)).to.be.true();
    });

    // Defaults are looked up by group name, so a member of a custom group has
    // none either.
    it('should refuse a hard Platform reset of a member of a group without defaults', async () => {
      useConfigWithoutDefaults('ops_1', 'ops');

      await expect(hardReset({ isPlatformOnlyReset: true }))
        .to.be.rejectedWith(/'ops_1'/);

      expect(dockerCompose.rm).to.not.have.been.called();
      expect(configFile.isConfigExists('ops_1')).to.be.true();
      expect(fs.existsSync(corePath)).to.be.true();
    });

    // The soft Platform reset changes no options, so it needs no defaults.
    it('should still reset Platform services and data without --hard', async () => {
      useConfigWithoutDefaults('devnet-ops');

      await reset({ isPlatformOnlyReset: true });

      expect(dockerCompose.rm).to.have.been.calledOnce();
      expect(configFile.isConfigExists('devnet-ops')).to.be.true();
      expect(fs.existsSync(platformPath)).to.be.true();
    });

    // A full hard reset removing such a config is what the operator asked for.
    it('should still remove the whole config on a full hard reset', async () => {
      useConfigWithoutDefaults('devnet-ops');

      await hardReset({ isPlatformOnlyReset: false });

      expect(configFile.isConfigExists('devnet-ops')).to.be.false();
      expect(fs.existsSync(homeDir.joinPath('devnet-ops'))).to.be.false();
    });
  });

  describe('a member of a group', () => {
    beforeEach(() => {
      const members = ['local_1', 'local_2'].map((name) => {
        const options = getLocalConfig().getStoredOptions();
        options.group = 'local';

        return new Config(name, options);
      });

      useConfigs(members);

      // Older homes also hold a config named after the group.
      writeFile('local', 'platform', 'drive', 'tenderdash', 'config.toml');
      writeFile('local_1', 'core', 'dash.conf');
      writeFile('local_1', 'platform', 'drive', 'tenderdash', 'config.toml');
      writeFile('local_2', 'core', 'dash.conf');
      writeFile('local_2', 'platform', 'drive', 'tenderdash', 'config.toml');
    });

    // Compose mounts each member's files from its own directory, named after
    // the config. The group name only selects the defaults.
    it('should remove its own Platform files and leave the other member\'s', async () => {
      await hardReset({ isPlatformOnlyReset: true });

      expect(fs.existsSync(homeDir.joinPath('local_1', 'platform'))).to.be.false();
      expect(fs.existsSync(homeDir.joinPath('local_1', 'core', 'dash.conf'))).to.be.true();
      expect(fs.existsSync(homeDir.joinPath('local_2', 'platform', 'drive', 'tenderdash', 'config.toml'))).to.be.true();
      expect(fs.existsSync(homeDir.joinPath('local', 'platform', 'drive', 'tenderdash', 'config.toml'))).to.be.true();
    });

    it('should remove only its own directory on a full hard reset', async () => {
      await hardReset({ isPlatformOnlyReset: false });

      expect(fs.existsSync(homeDir.joinPath('local_1'))).to.be.false();
      expect(fs.existsSync(homeDir.joinPath('local_2', 'core', 'dash.conf'))).to.be.true();
      expect(fs.existsSync(homeDir.joinPath('local', 'platform', 'drive', 'tenderdash', 'config.toml'))).to.be.true();
    });
  });
});
