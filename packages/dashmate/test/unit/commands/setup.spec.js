import fs from 'fs';
import os from 'os';
import path from 'path';
import { Listr } from 'listr2';
import SetupCommand from '../../../src/commands/setup.js';
import HomeDir from '../../../src/config/HomeDir.js';
import ConfigFile from '../../../src/config/configFile/ConfigFile.js';
import generateTenderdashNodeKey from '../../../src/tenderdash/generateTenderdashNodeKey.js';
import deriveTenderdashNodeId from '../../../src/tenderdash/deriveTenderdashNodeId.js';
import MuteOneLineError from '../../../src/oclif/errors/MuteOneLineError.js';
import createPlatformSetupConfigs from '../../fixtures/platformSetupConfigs.js';

describe('Setup command', () => {
  let homeDir;
  let secretsDir;
  let defaultConfigs;
  let setUp;
  let hardResetPlatform;
  let configFile;
  let dockerCompose;
  let setupPlatformTask;
  let getPlatformArtefacts;
  let artefacts;
  let platformContext;
  let exitCode;
  let nodeKey;
  let keyFile;
  let stderr;

  /**
   * @param {Config[]} configs
   */
  function useConfigs(configs) {
    configFile = new ConfigFile(configs, '5.2.0', 'abcdef12', null, null);
  }

  /**
   * @param {Object} [args]
   * @param {Object} [flags]
   * @return {Promise<Error|null>}
   */
  async function setup(args = {}, flags = {}, setupRegularPresetTask = () => new Listr([])) {
    try {
      await new SetupCommand().runWithDependencies(
        args,
        {
          verbose: false,
          platform: false,
          'non-interactive': false,
          'accept-unmet-requirements': false,
          ...flags,
        },
        configFile,
        () => new Listr([]),
        setupRegularPresetTask,
        dockerCompose,
        defaultConfigs,
        setupPlatformTask,
        getPlatformArtefacts,
        homeDir,
      );
    } catch (e) {
      return e instanceof MuteOneLineError ? e.getError() : e;
    }

    return null;
  }

  /**
   * An unattended Platform-only setup
   *
   * @param {Object} [flags]
   * @param {Object} [args]
   * @return {Promise<Error|null>}
   */
  function setupPlatform(flags = {}, args = { preset: 'testnet' }) {
    return setup(args, { platform: true, 'non-interactive': true, ...flags });
  }

  /**
   * @param {string} name
   * @param {string} content
   * @return {string}
   */
  function writeSecret(name, content) {
    const filePath = path.join(secretsDir, name);
    fs.writeFileSync(filePath, content, { mode: 0o600 });

    return filePath;
  }

  beforeEach(async function beforeEach() {
    homeDir = HomeDir.createTemp();
    secretsDir = fs.mkdtempSync(path.join(os.tmpdir(), 'dashmate-setup-secrets-'));
    ({ defaultConfigs, setUp, hardResetPlatform } = createPlatformSetupConfigs(homeDir));

    useConfigs([
      defaultConfigs.get('base'),
      defaultConfigs.get('mainnet'),
      await hardResetPlatform(setUp('testnet', { masternode: true })),
    ]);

    nodeKey = generateTenderdashNodeKey();
    keyFile = writeSecret('node-key', `${nodeKey}\n`);

    dockerCompose = { isServiceRunning: this.sinon.stub().resolves(true) };

    artefacts = { certificateFiles: [], dataVolumes: [], activeServices: [] };
    getPlatformArtefacts = this.sinon.stub().callsFake(async () => artefacts);

    platformContext = null;
    setupPlatformTask = this.sinon.stub().callsFake(() => new Listr([{
      task: (ctx) => {
        platformContext = ctx;
      },
    }]));

    exitCode = process.exitCode;

    stderr = '';
    this.sinon.stub(process.stderr, 'write').callsFake((chunk) => {
      stderr += chunk;

      return true;
    });
    this.sinon.stub(console, 'error').callsFake((text) => {
      stderr += `${text}\n`;
    });
  });

  afterEach(() => {
    process.exitCode = exitCode;
    homeDir.remove();
    fs.rmSync(secretsDir, { recursive: true, force: true });
  });

  describe('flags', () => {
    it('should refuse the local network flags with --platform', async () => {
      const error = await setupPlatform({ 'node-count': 4, 'enable-tor': false });

      expect(error.message).to.include('--node-count, --enable-tor cannot be used with --platform');
    });

    it('should refuse the Platform-only flags without --platform', async () => {
      const error = await setup({ preset: 'testnet' }, { 'ssl-provider': 'letsencrypt', config: 'testnet' });

      expect(error.message).to.include('--config, --ssl-provider work only with --platform');
    });

    it('should take --config as the preset', async () => {
      const error = await setupPlatform(
        { config: 'testnet', 'ssl-provider': 'letsencrypt', 'platform-node-key-file': keyFile },
        {},
      );

      expect(error).to.be.null();
      expect(platformContext.preset).to.equal('testnet');
    });

    it('should refuse a preset and --config that name different configs', async () => {
      const error = await setupPlatform({ config: 'mainnet' });

      expect(error.message).to.include('name different configs');
    });

    // Its identities, ports, chain and peers belong to the whole group, and
    // the seed node's wallet registers them.
    it('should refuse the local group and say how to set it up again', async () => {
      const error = await setupPlatform({}, { preset: 'local' });

      expect(error.message).to.include('dashmate group reset --group local --hard');
      expect(getPlatformArtefacts).to.not.have.been.called();
    });

    it('should need the preset when nothing names it and nobody can be asked', async () => {
      const error = await setupPlatform({}, {});

      expect(error.message).to.include('Name the preset');
    });

    // Reading stdin twice cannot give both answers.
    it('should let only one flag read stdin', async () => {
      const error = await setupPlatform({
        'platform-node-key-file': '-',
        'ssl-provider': 'zerossl',
        'zerossl-api-key-file': '-',
      });

      expect(error.message).to.include('Only one flag can read stdin');
    });
  });

  describe('preconditions, checked before anything runs', () => {
    it('should send a preset that was never set up to full setup', async () => {
      useConfigs([defaultConfigs.get('base'), defaultConfigs.get('testnet')]);

      const error = await setupPlatform({ 'ssl-provider': 'letsencrypt', 'platform-node-key-file': keyFile });

      expect(error.message).to.include('dashmate setup testnet');
      expect(getPlatformArtefacts).to.not.have.been.called();
      expect(setupPlatformTask).to.not.have.been.called();
    });

    it('should refuse a node whose Platform was configured, listing paths only', async () => {
      useConfigs([setUp('testnet', { masternode: true })]);

      const error = await setupPlatform({ 'ssl-provider': 'letsencrypt', 'platform-node-key-file': keyFile });

      expect(error.message).to.include('platform.drive.tenderdash.node.key')
        .and.include('dashmate reset --platform --hard --config testnet');
      expect(error.message).to.not.include(configFile.getConfig('testnet').get('platform.drive.tenderdash.node.key'));
      expect(setupPlatformTask).to.not.have.been.called();
    });

    it('should refuse after a reset that kept Platform data', async () => {
      artefacts.dataVolumes = ['dashmate_testnet_drive_abci_data'];

      const error = await setupPlatform({ 'ssl-provider': 'letsencrypt', 'platform-node-key-file': keyFile });

      expect(error.message).to.include('dashmate_testnet_drive_abci_data');
      expect(setupPlatformTask).to.not.have.been.called();
    });

    it('should refuse while Platform services are running', async () => {
      artefacts.activeServices = ['drive_tenderdash'];

      const error = await setupPlatform({ 'ssl-provider': 'letsencrypt', 'platform-node-key-file': keyFile });

      expect(error.message).to.include('drive_tenderdash')
        .and.include('dashmate stop --platform --config testnet');
      expect(setupPlatformTask).to.not.have.been.called();
    });

    // oclif hard-wraps the errors it prints, which splits the commands in
    // them, so refusals are printed as they are.
    it('should print a refusal as it is', async () => {
      artefacts.activeServices = ['drive_tenderdash'];

      const error = await setupPlatform({ 'ssl-provider': 'letsencrypt', 'platform-node-key-file': keyFile });

      expect(stderr).to.equal(`${error.message}\n`);
    });

    // "Nothing running" would be a guess.
    it('should refuse when Docker cannot be asked', async () => {
      getPlatformArtefacts.rejects(new Error('Docker is not running'));

      const error = await setupPlatform({ 'ssl-provider': 'letsencrypt', 'platform-node-key-file': keyFile });

      expect(error.message).to.include('Docker is not running');
      expect(setupPlatformTask).to.not.have.been.called();
    });
  });

  describe('answers given in advance', () => {
    it('should name every missing answer before Core is started or a provider contacted', async () => {
      const error = await setupPlatform();

      expect(error.message).to.include('--ssl-provider').and.include('--platform-node-key-file');
      expect(setupPlatformTask).to.not.have.been.called();
    });

    it('should need the ZeroSSL API key file for ZeroSSL', async () => {
      const error = await setupPlatform({ 'ssl-provider': 'zerossl', 'platform-node-key-file': keyFile });

      expect(error.message).to.include('--zerossl-api-key-file');
    });

    // update refuses a self-signed certificate on any masternode.
    it('should refuse a self-signed certificate for a masternode', async () => {
      const error = await setupPlatform({ 'ssl-provider': 'self-signed', 'platform-node-key-file': keyFile });

      expect(error.message).to.include('self-signed');
      expect(setupPlatformTask).to.not.have.been.called();
    });

    it('should refuse a malformed key file without quoting it', async () => {
      const junk = `${nodeKey.slice(0, 20)}!${nodeKey.slice(20)}`;

      const error = await setupPlatform({
        'ssl-provider': 'letsencrypt',
        'platform-node-key-file': writeSecret('junk-key', junk),
      });

      expect(error.message).to.include('--platform-node-key-file');
      expect(error.message).to.not.include(nodeKey.slice(0, 8));
    });

    // Whatever the file holds is sent to ZeroSSL.
    it('should refuse a ZeroSSL key file that does not hold an API key', async () => {
      const error = await setupPlatform({
        'ssl-provider': 'zerossl',
        'platform-node-key-file': writeSecret('node-key-2', nodeKey),
        'zerossl-api-key-file': keyFile,
      });

      expect(error.message).to.include('--zerossl-api-key-file does not hold a ZeroSSL API key');
      expect(error.message).to.not.include(nodeKey.slice(0, 8));
    });

    it('should refuse fullnode ports that collide before anything runs', async () => {
      const fullnode = await hardResetPlatform(setUp('testnet', { masternode: false }));
      useConfigs([fullnode]);

      const error = await setupPlatform({
        'ssl-provider': 'self-signed',
        'platform-p2p-port': fullnode.get('platform.gateway.listeners.dapiAndDrive.port'),
      });

      expect(error.message).to.include('the Platform P2P and HTTP ports are both');
      expect(setupPlatformTask).to.not.have.been.called();
    });

    it('should refuse an external IP that is not the node\'s', async () => {
      const error = await setupPlatform({
        'ssl-provider': 'letsencrypt',
        'platform-node-key-file': keyFile,
        'external-ip': '9.9.9.9',
      });

      expect(error.message).to.include('9.9.9.9').and.include('1.2.3.4');
    });

    it('should hold file certificate flags to what the prompt accepts', async () => {
      const error = await setupPlatform({
        'ssl-provider': 'file',
        'platform-node-key-file': keyFile,
        'ssl-chain-file': path.join(secretsDir, 'missing.crt'),
        'ssl-private-key-file': path.join(secretsDir, 'missing.key'),
      });

      expect(error.message).to.include('certificate chain file path is not valid');
    });

    it('should hand a complete set of answers to the task without prompting', async () => {
      const error = await setupPlatform({
        'ssl-provider': 'letsencrypt',
        'platform-node-key-file': keyFile,
        'platform-p2p-port': 36656,
      });

      expect(error).to.be.null();
      expect(process.exitCode).to.equal(0);
      expect(platformContext).to.include({
        preset: 'testnet',
        isMasternode: true,
        interactive: false,
        certificateProvider: 'letsencrypt',
        platformP2PPort: 36656,
        certificateRetryCommand: 'dashmate setup --platform --config testnet',
      });
      expect(platformContext.platformNodeKey).to.deep.equal({
        key: nodeKey,
        id: deriveTenderdashNodeId(nodeKey),
      });

      // Answers go to a copy, so a failed run cannot leave them behind.
      expect(platformContext.config).to.not.equal(configFile.getConfig('testnet'));
      expect(platformContext.config.get('platform.drive.tenderdash.mode')).to.equal('validator');
      expect(configFile.getConfig('testnet').get('platform.drive.tenderdash.mode')).to.equal('full');
    });

    it('should not need a key or an IP for a fullnode', async () => {
      const fullnode = await hardResetPlatform(setUp('testnet', { masternode: false }));
      fullnode.set('externalIp', '1.2.3.4');
      useConfigs([fullnode]);

      const error = await setupPlatform({ 'ssl-provider': 'self-signed' });

      expect(error).to.be.null();
      expect(platformContext.isMasternode).to.be.false();
    });

    it('should need an IP for a fullnode that has none', async () => {
      const fullnode = await hardResetPlatform(setUp('testnet', { masternode: false }));
      fullnode.set('externalIp', null);
      useConfigs([fullnode]);

      const error = await setupPlatform({ 'ssl-provider': 'self-signed' });

      expect(error.message).to.include('--external-ip');
    });

    it('should keep a mainnet fullnode on the mainnet ports', async () => {
      const fullnode = await hardResetPlatform(setUp('mainnet', { masternode: false }));
      useConfigs([fullnode]);

      const error = await setupPlatform({ 'ssl-provider': 'letsencrypt', 'platform-http-port': 1443 }, { preset: 'mainnet' });

      expect(error.message).to.include('a mainnet fullnode uses port 443');
    });
  });

  // Full setup asks at every step and cannot run unattended. Its steps read
  // this to decide whether they may prompt, and must not assume it.
  it('should state that full setup is interactive', async function it() {
    this.sinon.stub(console, 'log');
    let fullSetupContext;

    const error = await setup({ preset: 'mainnet' }, {}, () => new Listr([{
      task: (ctx) => {
        fullSetupContext = ctx;
      },
    }]));

    expect(error).to.be.null();
    expect(fullSetupContext.interactive).to.be.true();
  });

  // The command's cleanup on exit force-removes the containers it started.
  // The Core setup started runs on the node's chain database, so it is
  // stopped first the way `dashmate stop` stops it, with its grace period.
  describe('a Core that setup started', () => {
    let coreService;

    /**
     * @param {Error} [failure]
     */
    function startCoreInTask(failure = undefined) {
      setupPlatformTask.callsFake(() => new Listr([{
        task: (ctx) => {
          Object.assign(ctx, { coreService, isCoreStartedBySetup: true });

          if (failure) {
            throw failure;
          }
        },
      }], { renderer: 'silent' }));
    }

    beforeEach(function beforeEach() {
      coreService = { stop: this.sinon.stub().resolves(true) };
    });

    it('should be stopped gracefully when setup succeeds', async () => {
      startCoreInTask();

      const error = await setupPlatform({ 'ssl-provider': 'letsencrypt', 'platform-node-key-file': keyFile });

      expect(error).to.be.null();
      expect(coreService.stop).to.have.been.calledOnce();
    });

    it('should be stopped gracefully when setup fails', async () => {
      startCoreInTask(new Error('wrong key'));

      const error = await setupPlatform({ 'ssl-provider': 'letsencrypt', 'platform-node-key-file': keyFile });

      expect(error.message).to.equal('wrong key');
      expect(coreService.stop).to.have.been.calledOnce();
    });

    it('should leave a Core that was already running alone', async () => {
      setupPlatformTask.callsFake(() => new Listr([{
        task: (ctx) => {
          Object.assign(ctx, { coreService });
        },
      }], { renderer: 'silent' }));

      const error = await setupPlatform({ 'ssl-provider': 'letsencrypt', 'platform-node-key-file': keyFile });

      expect(error).to.be.null();
      expect(coreService.stop).to.not.have.been.called();
    });
  });

  describe('a run that fails', () => {
    let sslDir;

    /**
     * Make the setup task install a certificate, as a provider does, and fail
     * after it
     *
     * @param {Object} [ctxFields]
     */
    function failAfterInstalling({
      failure = new Error('Platform services were started while setup ran'),
      ...ctxFields
    } = {}) {
      setupPlatformTask.callsFake(() => new Listr([{
        task: (ctx) => {
          Object.assign(ctx, { isCertificateStepStarted: true }, ctxFields);
          fs.mkdirSync(sslDir, { recursive: true });
          fs.writeFileSync(path.join(sslDir, 'bundle.crt'), 'certificate');
          fs.writeFileSync(path.join(sslDir, 'private.key'), 'key');

          throw failure;
        },
      }], { renderer: 'silent' }));
    }

    beforeEach(() => {
      sslDir = homeDir.joinPath('testnet', 'platform', 'gateway', 'ssl');
    });

    // No option refers to them, so a second run would refuse over them.
    it('should remove the certificate files it installed, so it can run again', async () => {
      failAfterInstalling();

      const error = await setupPlatform({ 'ssl-provider': 'letsencrypt', 'platform-node-key-file': keyFile });

      expect(error.message).to.include('started while setup ran');
      expect(fs.existsSync(path.join(sslDir, 'bundle.crt'))).to.be.false();
      expect(fs.existsSync(path.join(sslDir, 'private.key'))).to.be.false();
      expect(stderr).to.include('were removed')
        .and.include('dashmate setup --platform --config testnet')
        .and.include('Let\'s Encrypt certificate already issued');
    });

    // No option refers to the certificate yet, so the next run would refuse it.
    it('should remove the certificate files when config.json could not be written, so a retry is accepted', async () => {
      failAfterInstalling({ failure: new Error('ENOSPC: no space left on device') });

      const error = await setupPlatform({ 'ssl-provider': 'letsencrypt', 'platform-node-key-file': keyFile });

      expect(error.message).to.include('ENOSPC');
      expect(process.exitCode).to.equal(1);
      expect(fs.existsSync(path.join(sslDir, 'bundle.crt'))).to.be.false();
      expect(stderr).to.include('nothing was saved');

      setupPlatformTask.callsFake(() => new Listr([]));
      expect(await setupPlatform({ 'ssl-provider': 'letsencrypt', 'platform-node-key-file': keyFile })).to.be.null();
    });

    // config.json refers to the certificate now: removing it would leave a
    // configured node without one.
    it('should keep the certificate when config.json was saved and rendering failed, and say how to render', async () => {
      failAfterInstalling({ isConfigSaved: true, failure: new Error('EACCES: permission denied') });

      const error = await setupPlatform({ 'ssl-provider': 'letsencrypt', 'platform-node-key-file': keyFile });

      expect(error.message).to.include('EACCES');
      expect(fs.existsSync(path.join(sslDir, 'bundle.crt'))).to.be.true();
      expect(stderr).to.include('dashmate config render --config testnet')
        .and.not.include('nothing was saved')
        .and.not.include('were removed');
    });

    // config.json could not be written, so nothing records the certificate.
    it('should say a ZeroSSL certificate it could not record needs cancelling', async () => {
      failAfterInstalling({ createdCertificate: true, certificate: { id: 'f'.repeat(32) } });

      await setupPlatform({ 'ssl-provider': 'letsencrypt', 'platform-node-key-file': keyFile });

      expect(stderr).to.include('nothing was saved')
        .and.include(`ZeroSSL created certificate ${'f'.repeat(32)}`);
      expect(fs.existsSync(path.join(sslDir, 'bundle.crt'))).to.be.false();
    });

    // ZeroSSL recorded its certificate in config.json, and its files are what
    // finishing needs.
    it('should keep a ZeroSSL certificate and say how to finish without a new one', async () => {
      failAfterInstalling({ createdCertificate: true, isZeroSslCertificateRecorded: true });

      await setupPlatform({ 'ssl-provider': 'letsencrypt', 'platform-node-key-file': keyFile });

      expect(fs.existsSync(path.join(sslDir, 'bundle.crt'))).to.be.true();
      expect(stderr).to.include('dashmate ssl obtain --config testnet');
      expect(stderr).to.not.include('--force');
    });
  });

  describe('full setup of a preset already set up', () => {
    // Before, it suggested `reset --hard`, which takes the Core part too.
    // A Regular masternode or a Core-only fullnode looks the same, so the
    // usual advice stays and Platform-only setup is offered next to it.
    it('should also offer setup --platform for a node whose Platform is not set up', () => {
      const text = SetupCommand.renderAlreadyConfigured(configFile.getConfig('testnet'), defaultConfigs);

      expect(text).to.include('dashmate setup --platform --config testnet')
        .and.include('dashmate reset --config testnet --hard');
    });

    it('should not offer setup --platform for a node whose Platform is configured', () => {
      const text = SetupCommand.renderAlreadyConfigured(setUp('testnet', { masternode: true }), defaultConfigs);

      expect(text).to.include('dashmate reset --config testnet --hard');
      expect(text).to.not.include('setup --platform');
    });
  });
});
