import HomeDir from '../../../src/config/HomeDir.js';
import getPlatformSetupState, {
  PLATFORM_SETUP_STATE,
  addReusableZeroSslCertificate,
  describePlatformSetupState,
} from '../../../src/config/getPlatformSetupState.js';
import createPlatformSetupConfigs from '../../fixtures/platformSetupConfigs.js';
import renderNodeKeyBackup from '../../../src/config/renderNodeKeyBackup.js';

describe('getPlatformSetupState', () => {
  let homeDir;
  let defaultConfigs;
  let setUp;
  let hardResetPlatform;

  /**
   * @param {Config} config
   * @param {Object} [artefacts]
   * @return {Object}
   */
  function stateOf(config, artefacts = {}) {
    return getPlatformSetupState(config, {
      defaultConfig: defaultConfigs.get(config.getName()),
      ...artefacts,
    });
  }

  beforeEach(() => {
    homeDir = HomeDir.createTemp();
    ({ defaultConfigs, setUp, hardResetPlatform } = createPlatformSetupConfigs(homeDir));
  });

  afterEach(() => {
    homeDir.remove();
  });

  describe('Core is not set up', () => {
    // Every preset is written to config.json on the first run, untouched, so
    // its presence says nothing about whether setup ran.
    it('should report an untouched preset as not set up', () => {
      ['mainnet', 'testnet'].forEach((preset) => {
        expect(stateOf(defaultConfigs.get(preset)).state)
          .to.equal(PLATFORM_SETUP_STATE.CORE_NOT_SET_UP);
      });
    });

    // Migrations touch untouched presets too (4.2.0 gave them a Tor control
    // password), so "differs from the defaults" cannot be the test.
    it('should still report a preset changed only by a migration as not set up', () => {
      const config = defaultConfigs.get('testnet');
      config.set('core.tor.control.password', 'generated-by-migration');

      expect(stateOf(config).state).to.equal(PLATFORM_SETUP_STATE.CORE_NOT_SET_UP);
    });

    it('should report a masternode without an operator key as not set up', () => {
      const config = setUp('testnet', { masternode: true });
      config.set('core.masternode.operator.privateKey', null);

      expect(stateOf(config).state).to.equal(PLATFORM_SETUP_STATE.CORE_NOT_SET_UP);
    });

    it('should report a masternode without an external IP as not set up', () => {
      const config = setUp('testnet', { masternode: true });
      config.set('externalIp', null);

      expect(stateOf(config).state).to.equal(PLATFORM_SETUP_STATE.CORE_NOT_SET_UP);
    });

    it('should point a preset that was never set up to full setup', () => {
      const text = describePlatformSetupState(stateOf(defaultConfigs.get('testnet')), 'testnet');

      expect(text).to.include('dashmate setup testnet');
    });
  });

  describe('after a hard Platform reset', () => {
    // The mainnet preset disables Platform, so the reset leaves an evonode
    // looking like a Core-only masternode.
    it('should report a mainnet evonode as unconfigured', async () => {
      const config = await hardResetPlatform(setUp('mainnet', { masternode: true }));

      expect(config.get('platform.enable')).to.be.false();
      expect(stateOf(config)).to.deep.equal({
        state: PLATFORM_SETUP_STATE.PLATFORM_UNCONFIGURED,
        markers: [],
      });
    });

    it('should report a testnet evonode as unconfigured', async () => {
      const config = await hardResetPlatform(setUp('testnet', { masternode: true }));

      expect(config.get('platform.enable')).to.be.true();
      expect(stateOf(config).state).to.equal(PLATFORM_SETUP_STATE.PLATFORM_UNCONFIGURED);
    });

    // The reset gives a fullnode a new key straight away. That key is not
    // registered anywhere, so it does not count as configuration.
    it('should report a testnet Evo fullnode with its generated key as unconfigured', async () => {
      const config = await hardResetPlatform(setUp('testnet', { masternode: false }));

      expect(config.get('platform.drive.tenderdash.node.key')).to.be.a('string');
      expect(stateOf(config).state).to.equal(PLATFORM_SETUP_STATE.PLATFORM_UNCONFIGURED);
    });

    // Whether it may run Platform is the registration's call, not this one's.
    it('should report a Regular masternode as unconfigured', () => {
      const config = setUp('testnet', { masternode: true, evo: false });

      expect(stateOf(config).state).to.equal(PLATFORM_SETUP_STATE.PLATFORM_UNCONFIGURED);
    });

    // Setup does not own these, so they are left as they are and do not count.
    it('should still report unconfigured after an image pin or a log level changed', async () => {
      const config = await hardResetPlatform(setUp('testnet', { masternode: true }));
      config.set('platform.drive.abci.docker.image', 'dashpay/drive:5.2.0-dev.1');
      config.set('platform.drive.tenderdash.log.level', 'debug');

      expect(stateOf(config).state).to.equal(PLATFORM_SETUP_STATE.PLATFORM_UNCONFIGURED);
    });
  });

  describe('Platform was configured', () => {
    it('should list the options setup owns, and never their values', () => {
      const config = setUp('testnet', { masternode: true });
      config.set('platform.gateway.ssl.provider', 'zerossl');
      config.set('platform.gateway.ssl.providerConfigs.zerossl.apiKey', 'zerossl-api-key-value-0123456789abcdef');

      const setupState = stateOf(config);

      expect(setupState.state).to.equal(PLATFORM_SETUP_STATE.PLATFORM_CONFIGURED);
      expect(setupState.markers).to.deep.equal([
        'platform.drive.tenderdash.node.key',
        'platform.gateway.ssl.enabled',
        'platform.gateway.ssl.providerConfigs.zerossl.apiKey',
      ]);

      const text = describePlatformSetupState(setupState, 'testnet');

      expect(text).to.not.include('zerossl-api-key-value');
      expect(text).to.not.include(config.get('platform.drive.tenderdash.node.key'));
      expect(text).to.include('dashmate reset --platform --hard --config testnet');
    });

    // The reset deletes the key and the registration names it, so the way
    // back starts with a backup, and setup is given that same file.
    it('should give the backup of a masternode key, and setup the backup file', () => {
      const text = describePlatformSetupState(stateOf(setUp('testnet', { masternode: true })), 'testnet');
      const { backupCommand, setupCommand } = renderNodeKeyBackup('testnet');

      expect(text).to.include(`    ${backupCommand}`)
        .and.include(`  ${setupCommand}`);
    });

    // Without a key the command writes "null" over the operator's backup.
    it('should not give a key backup when there is no key to back up', async () => {
      const config = await hardResetPlatform(setUp('testnet', { masternode: true }));
      config.set('platform.gateway.ssl.enabled', true);

      const text = describePlatformSetupState(stateOf(config), 'testnet');

      expect(text).to.include('platform.gateway.ssl.enabled');
      expect(text).to.not.include('config get');
    });

    // `ssl obtain` records the provider and enables TLS: running setup over it
    // would replace a certificate the operator chose.
    it('should report a node where ssl obtain ran after the reset as configured', async () => {
      const config = await hardResetPlatform(setUp('testnet', { masternode: false }));
      config.set('platform.gateway.ssl.enabled', true);

      expect(stateOf(config).state).to.equal(PLATFORM_SETUP_STATE.PLATFORM_CONFIGURED);
    });

    it('should report certificate files on disk as configured although no option names them', async () => {
      const config = await hardResetPlatform(setUp('testnet', { masternode: true }));
      const bundle = homeDir.joinPath('testnet', 'platform', 'gateway', 'ssl', 'bundle.crt');

      const setupState = stateOf(config, { certificateFiles: [bundle] });

      expect(setupState).to.deep.equal({
        state: PLATFORM_SETUP_STATE.PLATFORM_CONFIGURED,
        markers: [bundle],
      });

      // Left behind by a setup that failed after installing them, the usual
      // way they get there after a reset.
      expect(describePlatformSetupState(setupState, 'testnet')).to.include(`rm '${bundle}'`);
    });

    // `reset --platform --hard --keep-data` keeps the chain data: nothing of
    // Platform is meant to survive the reset setup follows.
    it('should report Platform data volumes left by --keep-data as configured', async () => {
      const config = await hardResetPlatform(setUp('testnet', { masternode: true }), { keepData: true });

      const setupState = stateOf(config, { dataVolumes: ['dashmate_testnet_drive_abci_data'] });

      expect(setupState.state).to.equal(PLATFORM_SETUP_STATE.PLATFORM_CONFIGURED);
      expect(setupState.markers).to.deep.equal(['Docker volume dashmate_testnet_drive_abci_data']);
      expect(describePlatformSetupState(setupState, 'testnet'))
        .to.include('dashmate reset --platform --config testnet');
    });
  });

  describe('an interrupted ZeroSSL setup', () => {
    // ZeroSSL saves config.json the moment it creates a certificate, so a
    // setup that stops at IP verification leaves every answer saved and TLS
    // off. Setting up again cannot finish that certificate.
    it('should be recognised, and its finishing commands given', async () => {
      const config = await hardResetPlatform(setUp('testnet', { masternode: true }));
      config.set('platform.drive.tenderdash.node.key', setUp('testnet', { masternode: true }).get('platform.drive.tenderdash.node.key'));
      config.set('platform.gateway.ssl.provider', 'zerossl');
      config.set('platform.gateway.ssl.providerConfigs.zerossl.apiKey', 'k'.repeat(32));
      config.set('platform.gateway.ssl.providerConfigs.zerossl.id', 'c'.repeat(32));

      const setupState = stateOf(config);

      expect(setupState.state).to.equal(PLATFORM_SETUP_STATE.ZEROSSL_INTERRUPTED);

      // The key existed only in memory, so the certificate cannot be used.
      const text = describePlatformSetupState({
        ...setupState,
        reusableCertificate: {
          isReusable: false,
          status: 'pending_validation',
          reason: 'the private key it was requested with is not on this node',
        },
      }, 'testnet');

      expect(text).to.include('dashmate ssl obtain --force --config testnet')
        .and.include('the private key it was requested with is not on this node');
      // `ssl cleanup` cancels every unfinished certificate of the account.
      expect(text).to.include(`cancel certificate ${'c'.repeat(32)} in the ZeroSSL dashboard`);
    });

    // A second --force would spend another certificate when a valid pair is
    // already installed; a plain obtain finds it and finishes.
    it('should finish without a new certificate when the issued pair is on disk', async () => {
      const config = await hardResetPlatform(setUp('testnet', { masternode: true }));
      config.set('platform.drive.tenderdash.node.key', setUp('testnet', { masternode: true }).get('platform.drive.tenderdash.node.key'));
      config.set('platform.gateway.ssl.provider', 'zerossl');
      config.set('platform.gateway.ssl.providerConfigs.zerossl.apiKey', 'k'.repeat(32));
      config.set('platform.gateway.ssl.providerConfigs.zerossl.id', 'c'.repeat(32));
      const sslDir = homeDir.joinPath('testnet', 'platform', 'gateway', 'ssl');

      const setupState = stateOf(config, {
        certificateFiles: [`${sslDir}/bundle.crt`, `${sslDir}/private.key`],
        certificate: { isUsable: true, directories: [] },
      });
      const text = describePlatformSetupState(setupState, 'testnet');

      expect(setupState.state).to.equal(PLATFORM_SETUP_STATE.ZEROSSL_INTERRUPTED);
      expect(text).to.include('dashmate ssl obtain --config testnet');
      expect(text).to.not.include('--force');
    });

    // A run that stopped while writing them, out of disk space say, leaves
    // both names on disk. A plain obtain finds them there and would enable
    // TLS without writing them again.
    it('should not call the certificate installed when its files are there but cannot be served', async () => {
      const config = await hardResetPlatform(setUp('testnet', { masternode: true }));
      config.set('platform.drive.tenderdash.node.key', setUp('testnet', { masternode: true }).get('platform.drive.tenderdash.node.key'));
      config.set('platform.gateway.ssl.provider', 'zerossl');
      config.set('platform.gateway.ssl.providerConfigs.zerossl.apiKey', 'k'.repeat(32));
      config.set('platform.gateway.ssl.providerConfigs.zerossl.id', 'c'.repeat(32));
      const sslDir = homeDir.joinPath('testnet', 'platform', 'gateway', 'ssl');

      const setupState = stateOf(config, {
        certificateFiles: [`${sslDir}/bundle.crt`, `${sslDir}/private.key`],
        certificate: { isUsable: false, directories: [] },
      });

      expect(setupState.isCertificateInstalled).to.be.false();
    });

    // Whether the gateway can serve the files and whether ZeroSSL can send
    // the certificate again are two questions. ZeroSSL saves the certificate
    // ID before verification and the files are written last.
    describe('whose certificate the gateway cannot serve', () => {
      let config;
      let findReusableZeroSslCertificate;

      /**
       * @return {Promise<string>} the advice
       */
      async function advice() {
        const setupState = await addReusableZeroSslCertificate(
          stateOf(config, { certificate: { isUsable: false, directories: [] } }),
          config,
          findReusableZeroSslCertificate,
        );

        return describePlatformSetupState(setupState, 'testnet');
      }

      beforeEach(async function beforeEach() {
        config = await hardResetPlatform(setUp('testnet', { masternode: true }));
        config.set('platform.drive.tenderdash.node.key', setUp('testnet', { masternode: true }).get('platform.drive.tenderdash.node.key'));
        config.set('platform.gateway.ssl.provider', 'zerossl');
        config.set('platform.gateway.ssl.providerConfigs.zerossl.apiKey', 'k'.repeat(32));
        config.set('platform.gateway.ssl.providerConfigs.zerossl.id', 'c'.repeat(32));

        findReusableZeroSslCertificate = this.sinon.stub();
      });

      // A full disk stopped the bundle's write after the key was written.
      it('should install an issued certificate again rather than create one', async () => {
        findReusableZeroSslCertificate.resolves({ isReusable: true, status: 'issued', reason: null });

        const text = await advice();

        expect(findReusableZeroSslCertificate).to.have.been.calledOnceWith(config);
        expect(text).to.include('dashmate ssl obtain --config testnet')
          .and.include('ZeroSSL issued its certificate')
          .and.include('c'.repeat(32))
          .and.not.include('--force')
          .and.not.include('port 80');
      });

      it('should finish a certificate ZeroSSL still validates rather than create one', async () => {
        findReusableZeroSslCertificate.resolves({ isReusable: true, status: 'pending_validation', reason: null });

        const text = await advice();

        expect(text).to.include('open inbound TCP port 80')
          .and.include('dashmate ssl obtain --config testnet')
          .and.not.include('--force');
      });

      ['cancelled', 'expired'].forEach((status) => {
        it(`should create a new certificate when ZeroSSL reports the saved one ${status}`, async () => {
          findReusableZeroSslCertificate.resolves({
            isReusable: false, status, reason: `ZeroSSL reports it ${status}`,
          });

          expect(await advice()).to.include('dashmate ssl obtain --force --config testnet')
            .and.include(`ZeroSSL reports it ${status}`);
        });
      });

      it('should create a new certificate when no certificate ID is saved', () => {
        const text = describePlatformSetupState({
          ...stateOf(config, { certificate: { isUsable: false, directories: [] } }),
          reusableCertificate: { isReusable: false, status: null, reason: 'no certificate ID is saved' },
        }, 'testnet');

        expect(text).to.include('dashmate ssl obtain --force --config testnet');
      });

      // A plain obtain decides by the same validation, and asks for a new
      // certificate only when the saved one cannot be used.
      it('should not ask for a new certificate when ZeroSSL could not be asked', async () => {
        findReusableZeroSslCertificate.resolves({ isReusable: null, status: null, reason: 'fetch failed' });

        const text = await advice();

        expect(text).to.include('could not ask ZeroSSL')
          .and.include('fetch failed')
          .and.include('dashmate ssl obtain --config testnet')
          .and.not.include('--force');
      });

      it('should not ask ZeroSSL when the gateway can serve the installed certificate', async () => {
        await addReusableZeroSslCertificate(
          stateOf(config, { certificate: { isUsable: true, directories: [] } }),
          config,
          findReusableZeroSslCertificate,
        );

        expect(findReusableZeroSslCertificate).to.not.have.been.called();
      });
    });
  });
});
