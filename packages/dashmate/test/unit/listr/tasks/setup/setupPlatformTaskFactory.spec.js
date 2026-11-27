import { Listr } from 'listr2';
import lodash from 'lodash';
import HomeDir from '../../../../../src/config/HomeDir.js';
import Config from '../../../../../src/config/Config.js';
import ConfigFile from '../../../../../src/config/configFile/ConfigFile.js';
import setupPlatformTaskFactory from '../../../../../src/listr/tasks/setup/platform/setupPlatformTaskFactory.js';
import { REGISTRATION_STATUS } from '../../../../../src/core/readPlatformRegistration.js';
import generateTenderdashNodeKey from '../../../../../src/tenderdash/generateTenderdashNodeKey.js';
import deriveTenderdashNodeId from '../../../../../src/tenderdash/deriveTenderdashNodeId.js';
import getEnquirerMock from '../../../../../src/test/mock/getEnquirerMock.js';
import { NODE_TYPE_FULLNODE, NODE_TYPE_MASTERNODE, PLATFORM_ONLY_RENDER_SCOPE } from '../../../../../src/constants.js';
import createPlatformSetupConfigs from '../../../../fixtures/platformSetupConfigs.js';
import getConfigProfilesFactory from '../../../../../src/config/getConfigProfilesFactory.js';
import getPlatformSetupState, { PLATFORM_SETUP_STATE } from '../../../../../src/config/getPlatformSetupState.js';

// The options Platform-only setup may write (spec: only these, plus externalIp
// from null on a fullnode).
const SETUP_OWNED_PATHS = [
  'platform.enable',
  'platform.drive.tenderdash.mode',
  'platform.drive.tenderdash.node.',
  'platform.drive.tenderdash.p2p.port',
  'platform.gateway.listeners.dapiAndDrive.port',
  'platform.gateway.ssl.',
];

/**
 * @param {Object} object
 * @param {string} [prefix]
 * @return {Object} path -> value
 */
function flatten(object, prefix = '') {
  return Object.entries(object).reduce((flat, [key, value]) => {
    const path = `${prefix}${key}`;

    if (lodash.isPlainObject(value)) {
      return { ...flat, ...flatten(value, `${path}.`) };
    }

    return { ...flat, [path]: value };
  }, {});
}

describe('setupPlatformTask', () => {
  let homeDir;
  let defaultConfigs;
  let setUp;
  let hardResetPlatform;
  let configFile;
  let config;
  let registeredKey;
  let registration;
  let dockerCompose;
  let startCore;
  let rpcClient;
  let createRpcClient;
  let waitForCoreSync;
  let readPlatformRegistration;
  let configureSSLCertificateTask;
  let getPlatformArtefacts;
  let configFileRepository;
  let writeConfigTemplates;
  let setupPlatformTask;
  let sandbox;

  /**
   * Run the task as the setup command does, on a copy of the stored config
   *
   * @param {Object} [answers] - context fields
   * @param {Array} [prompts] - answers for an interactive run
   * @return {Promise<{ ctx: Object, error: Error|null, enquirer: Object }>}
   */
  async function run(answers = {}, prompts = undefined) {
    const isMasternode = config.get('core.masternode.enable');
    const setupConfig = new Config(config.getName(), config.getStoredOptions());
    setupConfig.set('platform.enable', true);
    setupConfig.set('platform.drive.tenderdash.mode', isMasternode ? 'validator' : 'full');

    const ctx = {
      preset: config.getName(),
      config: setupConfig,
      storedConfig: config,
      isMasternode,
      nodeType: isMasternode ? NODE_TYPE_MASTERNODE : NODE_TYPE_FULLNODE,
      isHP: true,
      interactive: prompts !== undefined,
      wasPlatformEnabled: config.get('platform.enable'),
      isCoreRunning: true,
      certificateRetryCommand: `dashmate setup --platform --config ${config.getName()}`,
      certificateProvider: 'letsencrypt',
      ...answers,
    };

    const enquirer = getEnquirerMock(sandbox, ...(prompts ?? []));

    const tasks = new Listr([{ task: () => setupPlatformTask() }], { renderer: 'silent' });
    tasks.options.injectWrapper = { enquirer };

    let error = null;
    try {
      await tasks.run(ctx);
    } catch (e) {
      error = e;
    }

    return { ctx, error, enquirer };
  }

  /**
   * @return {Config} what the command would save
   */
  function saved() {
    return configFile.getConfig(config.getName());
  }

  /**
   * @param {Config} reset
   */
  function useConfig(reset) {
    config = reset;
    configFile = new ConfigFile([config], '5.2.0', 'abcdef12', null, null);
    // A loaded config file starts unchanged.
    config.markAsSaved();
    configFile.markAsSaved();

    setupPlatformTask = setupPlatformTaskFactory(
      configFile,
      dockerCompose,
      startCore,
      createRpcClient,
      async () => '127.0.0.1',
      waitForCoreSync,
      readPlatformRegistration,
      () => new Listr([]),
      configureSSLCertificateTask,
      getPlatformArtefacts,
      defaultConfigs,
      configFileRepository,
      writeConfigTemplates,
    );
  }

  beforeEach(async function beforeEach() {
    sandbox = this.sinon;

    homeDir = HomeDir.createTemp();
    ({ defaultConfigs, setUp, hardResetPlatform } = createPlatformSetupConfigs(homeDir));

    registeredKey = generateTenderdashNodeKey();
    registration = {
      status: REGISTRATION_STATUS.EVO,
      proTxHash: 'ab'.repeat(32),
      platformNodeId: deriveTenderdashNodeId(registeredKey),
      platformP2PPort: 36656,
      platformHttpPort: 1443,
      poSeBanHeight: null,
    };

    rpcClient = { name: 'running Core' };
    dockerCompose = { isServiceRunning: this.sinon.stub().resolves(true) };
    startCore = this.sinon.stub().resolves({ getRpcClient: () => ({ name: 'started Core' }) });
    createRpcClient = this.sinon.stub().returns(rpcClient);
    waitForCoreSync = this.sinon.stub().resolves();
    readPlatformRegistration = this.sinon.stub().callsFake(async () => registration);

    // What a successful certificate step leaves: a provider, and TLS on.
    configureSSLCertificateTask = this.sinon.stub().callsFake(() => new Listr([{
      task: (ctx) => {
        ctx.config.set('platform.gateway.ssl.provider', ctx.certificateProvider);
        ctx.config.set('platform.gateway.ssl.enabled', true);
      },
    }]));

    getPlatformArtefacts = { getActiveServices: this.sinon.stub().resolves([]) };
    configFileRepository = { write: this.sinon.stub() };
    writeConfigTemplates = this.sinon.stub();

    useConfig(await hardResetPlatform(setUp('testnet', { masternode: true })));
  });

  afterEach(() => {
    homeDir.remove();
  });

  describe('an evonode', () => {
    it('should install the registered identity and endpoints', async () => {
      const { error } = await run({ platformNodeKey: { key: registeredKey, id: registration.platformNodeId } });

      expect(error).to.be.null();
      expect(saved().get('platform.enable')).to.be.true();
      expect(saved().get('platform.drive.tenderdash.mode')).to.equal('validator');
      expect(saved().get('platform.drive.tenderdash.node')).to.deep.equal({
        key: registeredKey,
        id: registration.platformNodeId,
      });
      expect(saved().get('platform.drive.tenderdash.p2p.port')).to.equal(36656);
      expect(saved().get('platform.gateway.listeners.dapiAndDrive.port')).to.equal(1443);
    });

    // Validators authenticate node IDs when they dial each other: any other
    // key leaves the node unable to take part, so there is no way past this.
    it('should refuse a key that is not the registered one, and save nothing', async () => {
      const other = generateTenderdashNodeKey();

      const { error } = await run({ platformNodeKey: { key: other, id: deriveTenderdashNodeId(other) } });

      expect(error.message).to.include(registration.platformNodeId)
        .and.include(deriveTenderdashNodeId(other))
        .and.include('ProUpServTx');
      expect(configFile.isChanged()).to.be.false();
      expect(configureSSLCertificateTask).to.not.have.been.called();
    });

    it('should read the registration from the running Core', async () => {
      await run({ platformNodeKey: { key: registeredKey, id: registration.platformNodeId } });

      expect(startCore).to.not.have.been.called();
      expect(readPlatformRegistration).to.have.been.calledOnceWith(rpcClient, {
        operatorPrivateKey: config.get('core.masternode.operator.privateKey'),
        externalIp: config.get('externalIp'),
      });
    });

    // The command stops the Core it started when it exits, success or not.
    it('should start Core when it is stopped, and say Core must be started afterwards', async () => {
      dockerCompose.isServiceRunning.resolves(false);

      const { ctx, error } = await run({
        platformNodeKey: { key: registeredKey, id: registration.platformNodeId },
        isCoreRunning: false,
      });

      expect(error).to.be.null();
      expect(startCore).to.have.been.calledOnce();
      expect(waitForCoreSync).to.have.been.calledOnce();
      expect(readPlatformRegistration.firstCall.args[0]).to.deep.equal({ name: 'started Core' });
      expect(ctx.isCoreRunning).to.be.false();
    });

    // Compose 2.24 creates the volumes of every service its profiles enable,
    // `run core` included, and labels them as the project's. A Platform volume
    // left by a run that then failed would refuse the next run, as if a reset
    // had kept Platform data.
    it('should start Core alone, so a run that fails can simply run again', async () => {
      dockerCompose.isServiceRunning.resolves(false);
      config.set('platform.quorumList.enabled', true);
      useConfig(config);

      const createdVolumes = [];
      startCore.callsFake(async (coreConfig) => {
        if (getConfigProfilesFactory()(coreConfig).some((profile) => profile.startsWith('platform'))) {
          createdVolumes.push('dashmate_testnet_drive_abci_data');
        }

        return { getRpcClient: () => ({ name: 'started Core' }) };
      });

      const other = generateTenderdashNodeKey();
      const { error } = await run({
        platformNodeKey: { key: other, id: deriveTenderdashNodeId(other) },
        isCoreRunning: false,
      });

      expect(error.message).to.include(registration.platformNodeId);
      expect(startCore).to.have.been.calledOnce();
      expect(createdVolumes).to.be.empty();

      // What the retry's precondition check finds
      const retryState = getPlatformSetupState(config, {
        defaultConfig: defaultConfigs.get('testnet'),
        dataVolumes: createdVolumes,
      });
      expect(retryState.state).to.equal(PLATFORM_SETUP_STATE.PLATFORM_UNCONFIGURED);
    });

    it('should not start Core when the operator declines', async () => {
      dockerCompose.isServiceRunning.resolves(false);

      const { error } = await run({}, [false]);

      expect(error.message).to.include('needs Core');
      expect(startCore).to.not.have.been.called();
    });

    it('should accept port answers equal to the registration, so a script can repeat them', async () => {
      const { error } = await run({
        platformNodeKey: { key: registeredKey, id: registration.platformNodeId },
        platformP2PPort: 36656,
        platformHttpPort: 1443,
      });

      expect(error).to.be.null();
    });

    it('should refuse a port answer that differs from the registration', async () => {
      const { error } = await run({
        platformNodeKey: { key: registeredKey, id: registration.platformNodeId },
        platformP2PPort: 26656,
      });

      expect(error.message).to.include('--platform-p2p-port 26656').and.include('36656');
      expect(configFile.isChanged()).to.be.false();
    });

    [
      [REGISTRATION_STATUS.REGULAR, { proTxHash: 'cd'.repeat(32) }, 'Regular'],
      [REGISTRATION_STATUS.NOT_FOUND, {}, 'core.masternode.operator.privateKey'],
      [REGISTRATION_STATUS.UNSUPPORTED_ENDPOINTS, { reason: 'The registered Platform P2P address 5.6.7.8:26656 is not on this node\'s external IP' }, '5.6.7.8:26656'],
      [REGISTRATION_STATUS.UNAVAILABLE, { reason: 'Core has not finished syncing' }, 'syncing'],
    ].forEach(([status, fields, expected]) => {
      it(`should refuse a ${status} registration and save nothing`, async () => {
        registration = { status, ...fields };

        const { error } = await run({
          platformNodeKey: { key: registeredKey, id: deriveTenderdashNodeId(registeredKey) },
        });

        expect(error.message).to.include(expected);
        expect(configFile.isChanged()).to.be.false();
      });
    });

    // A banned node has to be configured before it can be revived.
    it('should set up a PoSe-banned evonode and say how to revive it', async () => {
      registration.poSeBanHeight = 1310;

      const { error } = await run({ platformNodeKey: { key: registeredKey, id: registration.platformNodeId } });

      expect(error).to.be.null();
      expect(saved().get('platform.drive.tenderdash.node.key')).to.equal(registeredKey);
    });

    describe('asked for the key', () => {
      // The verbose renderer logs prompt redraws, so a plain prompt would
      // print the key under --verbose.
      it('should mask the key and prefill nothing', async () => {
        const { error, enquirer } = await run({}, [registeredKey]);

        expect(error).to.be.null();

        const [prompt] = enquirer.options;
        expect(prompt.type).to.equal('password');
        expect(prompt.initial).to.be.undefined();
        expect(prompt.header).to.include(registration.platformNodeId);
      });

      it('should ask again for a key that is not the registered one, naming both IDs', async () => {
        const { enquirer } = await run({}, [registeredKey]);
        const other = generateTenderdashNodeKey();

        const validity = enquirer.options[0].validate(other);

        expect(validity).to.include(deriveTenderdashNodeId(other))
          .and.include(registration.platformNodeId);
        expect(enquirer.options[0].validate(registeredKey)).to.be.true();
        expect(enquirer.options[0].validate('not a key')).to.match(/base64/);
      });
    });
  });

  describe('an Evo fullnode', () => {
    let storedKey;

    beforeEach(async () => {
      useConfig(await hardResetPlatform(setUp('testnet', { masternode: false })));
      storedKey = config.get('platform.drive.tenderdash.node.key');
    });

    it('should keep the stored key unless told otherwise', async () => {
      const { error } = await run();

      expect(error).to.be.null();
      expect(saved().get('platform.drive.tenderdash.node.key')).to.equal(storedKey);
      expect(readPlatformRegistration).to.not.have.been.called();
    });

    it('should use the key it is given', async () => {
      const key = generateTenderdashNodeKey();

      await run({ platformNodeKey: { key, id: deriveTenderdashNodeId(key) } });

      expect(saved().get('platform.drive.tenderdash.node')).to.deep.equal({
        key,
        id: deriveTenderdashNodeId(key),
      });
    });

    it('should offer to keep the stored key by default', async () => {
      const { enquirer } = await run({}, ['keep', { platformP2PPort: '36656', platformHTTPPort: '1443' }]);

      const [choice] = enquirer.options;
      expect(choice.initial).to.equal('keep');
      expect(choice.choices[0].message).to.include(config.get('platform.drive.tenderdash.node.id'));
      expect(choice.choices[0].message).to.not.include(storedKey.slice(0, 8));
      expect(saved().get('platform.drive.tenderdash.node.key')).to.equal(storedKey);
    });

    // A mainnet hard reset disables Platform, and then no key is generated.
    it('should generate a key when none is stored', async () => {
      useConfig(await hardResetPlatform(setUp('mainnet', { masternode: false })));
      expect(config.get('platform.drive.tenderdash.node.key')).to.be.null();

      await run();

      const { key, id } = saved().get('platform.drive.tenderdash.node');
      expect(key).to.be.a('string');
      expect(id).to.equal(deriveTenderdashNodeId(key));
    });

    it('should take the ports it is given', async () => {
      await run({ platformP2PPort: 36656, platformHttpPort: 2443 });

      expect(saved().get('platform.drive.tenderdash.p2p.port')).to.equal(36656);
      expect(saved().get('platform.gateway.listeners.dapiAndDrive.port')).to.equal(2443);
    });

    // The schema lets a stored 0 through, which would be advertised as IP:0.
    it('should refuse a stored port that cannot be used', async () => {
      config.set('platform.drive.tenderdash.p2p.port', 0);
      useConfig(config);

      const { error } = await run();

      expect(error.message).to.include('Platform P2P port 0 cannot be used');
      expect(configureSSLCertificateTask).to.not.have.been.called();
    });

    it('should refuse Platform ports that Core already uses', async () => {
      const { error } = await run({ platformP2PPort: config.get('core.p2p.port') });

      expect(error.message).to.include('Core already uses port');
    });

    it('should ask for the external IP only when the node has none', async () => {
      await run({ externalIpAnswer: '5.6.7.8' });
      expect(saved().get('externalIp')).to.equal('1.2.3.4');

      config.set('externalIp', null);
      useConfig(config);

      await run({ externalIpAnswer: '5.6.7.8' });
      expect(saved().get('externalIp')).to.equal('5.6.7.8');
    });
  });

  // Setup does not own the rest: image pins, log levels, Core.
  it('should change only the options Platform setup owns', async () => {
    config.set('platform.drive.abci.docker.image', 'dashpay/drive:pinned');
    useConfig(config);
    const before = flatten(config.getStoredOptions());

    await run({ platformNodeKey: { key: registeredKey, id: registration.platformNodeId } });

    const after = flatten(saved().getStoredOptions());
    const changed = Object.keys({ ...before, ...after })
      .filter((path) => !lodash.isEqual(before[path], after[path]));

    expect(changed).to.not.be.empty();
    changed.forEach((path) => {
      expect(SETUP_OWNED_PATHS.some((owned) => path === owned || path.startsWith(owned)), path)
        .to.be.true();
    });
  });

  // `start` does not take the config lock, so Platform may have been started
  // while setup waited on the certificate.
  // Compose gets every option in its environment, and debug output of child
  // processes prints it: the node key given to setup must not be there.
  it('should look for running services with the config as it was', async () => {
    await run({ platformNodeKey: { key: registeredKey, id: registration.platformNodeId } });

    const [checked] = getPlatformArtefacts.getActiveServices.firstCall.args;
    expect(checked).to.equal(config);
    expect(checked.get('platform.drive.tenderdash.node.key')).to.be.null();
  });

  // Saved and rendered before success is reported, not when the command
  // exits: a failure is then a failed setup, which removes the certificate
  // files this run installed, so the next run is not refused over them.
  describe('saving', () => {
    const answers = () => ({ platformNodeKey: { key: registeredKey, id: registration.platformNodeId } });

    it('should write config.json and render the service files before it reports success', async () => {
      // What reaches the disk is what the config file holds when it is
      // written, not what it holds once the task has finished.
      let written;
      configFileRepository.write.callsFake((file) => {
        written = file.toObject().configs[config.getName()];
      });

      const { ctx, error } = await run(answers());

      expect(error).to.be.null();
      expect(configFileRepository.write).to.have.been.calledOnceWithExactly(configFile);
      expect(written.platform.drive.tenderdash.node).to.deep.equal({
        key: registeredKey,
        id: registration.platformNodeId,
      });
      expect(written.platform.gateway.ssl.enabled).to.be.true();
      expect(writeConfigTemplates).to.have.been.calledOnce();
      expect(writeConfigTemplates.firstCall.args[0]).to.equal(ctx.config);
      expect(writeConfigTemplates).to.have.been.calledAfter(configFileRepository.write);
      expect(saved()).to.equal(ctx.config);
      expect(ctx.isConfigSaved).to.be.true();
    });

    it('should fail, with nothing rendered, when config.json cannot be written', async () => {
      configFileRepository.write.throws(new Error('ENOSPC: no space left on device'));

      const { ctx, error } = await run(answers());

      expect(error.message).to.include('ENOSPC');
      expect(ctx.isConfigSaved).to.not.equal(true);
      expect(writeConfigTemplates).to.not.have.been.called();
    });

    // Core's files read no Platform option. Rendered again, dash.conf would
    // lose an operator's edit and get fresh rpcauth salts under a running Core.
    it('should leave Core\'s service files as they are when it changed only Platform options', async () => {
      const { error } = await run(answers());

      expect(error).to.be.null();
      expect(writeConfigTemplates.firstCall.args[1]).to.deep.equal(PLATFORM_ONLY_RENDER_SCOPE);
    });

    // dash.conf advertises externalIp.
    it('should render Core\'s service files too when it set the external IP', async () => {
      config = await hardResetPlatform(setUp('testnet', { masternode: false }));
      config.set('externalIp', null);
      useConfig(config);

      const { error } = await run({ externalIpAnswer: '5.6.7.8' });

      expect(error).to.be.null();
      expect(saved().get('externalIp')).to.equal('5.6.7.8');
      expect(writeConfigTemplates.firstCall.args[1]).to.be.undefined();
    });

    // config.json refers to the certificate now, so it must stay.
    it('should fail, recording that config.json was saved, when rendering fails', async () => {
      writeConfigTemplates.throws(new Error('EACCES: permission denied'));

      const { ctx, error } = await run(answers());

      expect(error.message).to.include('EACCES');
      expect(ctx.isConfigSaved).to.be.true();
    });
  });

  it('should save nothing when Platform services were started meanwhile', async () => {
    getPlatformArtefacts.getActiveServices.resolves(['drive_abci']);

    const { error } = await run({ platformNodeKey: { key: registeredKey, id: registration.platformNodeId } });

    expect(error.message).to.include('drive_abci').and.include('dashmate stop --platform --config testnet');
    expect(configFile.isChanged()).to.be.false();
    expect(configFileRepository.write).to.not.have.been.called();
  });

  it('should not print the node key anywhere in what it reports', async () => {
    const { ctx } = await run({ platformNodeKey: { key: registeredKey, id: registration.platformNodeId } });

    const other = generateTenderdashNodeKey();
    const { error } = await run({ platformNodeKey: { key: other, id: deriveTenderdashNodeId(other) } });

    const reported = JSON.stringify({
      registration: ctx.registration,
      error: `${error.message}\n${error.stack}`,
    });

    for (let i = 0; i + 8 <= registeredKey.length; i += 1) {
      expect(reported).to.not.include(registeredKey.slice(i, i + 8));
      expect(reported).to.not.include(other.slice(i, i + 8));
    }
  });
});
