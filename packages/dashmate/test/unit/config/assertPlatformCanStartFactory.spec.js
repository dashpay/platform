import fs from 'fs';
import path from 'path';
import HomeDir from '../../../src/config/HomeDir.js';
import Config from '../../../src/config/Config.js';
import assertPlatformCanStartFactory from '../../../src/config/assertPlatformCanStartFactory.js';
import generateTenderdashNodeKey from '../../../src/tenderdash/generateTenderdashNodeKey.js';
import deriveTenderdashNodeId from '../../../src/tenderdash/deriveTenderdashNodeId.js';
import createPlatformSetupConfigs from '../../fixtures/platformSetupConfigs.js';
import { issueCertificate } from '../../../src/test/certificateFixtures.js';
import validateZeroSslCertificateFactory from '../../../src/ssl/zerossl/validateZeroSslCertificateFactory.js';
import findReusableZeroSslCertificateFactory from '../../../src/ssl/zerossl/findReusableZeroSslCertificateFactory.js';
import Certificate from '../../../src/ssl/zerossl/Certificate.js';
import ProviderUnreachableError from '../../../src/ssl/errors/ProviderUnreachableError.js';

describe('assertPlatformCanStart', () => {
  let homeDir;
  let defaultConfigs;
  let setUp;
  let hardResetPlatform;
  let assertPlatformCanStart;
  let getCertificate;

  /**
   * Write node_key.json as rendering does
   *
   * @param {Config} config
   * @param {string|null} id
   * @param {string|null} key
   */
  function renderNodeKey(config, { id, key } = config.get('platform.drive.tenderdash.node')) {
    const filePath = homeDir.joinPath(config.getName(), 'platform', 'drive', 'tenderdash', 'node_key.json');
    fs.mkdirSync(path.dirname(filePath), { recursive: true });
    fs.writeFileSync(filePath, `{
  "id": "${id}",
  "priv_key": {
    "type": "tendermint/PrivKeyEd25519",
    "value": "${key}"
  }
}`);
  }

  /**
   * @param {Config} config
   * @return {Promise<Error|null>}
   */
  async function refusalOf(config) {
    try {
      await assertPlatformCanStart(config);
    } catch (e) {
      return e;
    }

    return null;
  }

  /**
   * What ZeroSSL answers about the saved certificate
   *
   * @param {string} status
   * @return {Certificate}
   */
  function zeroSslCertificate(status) {
    const pad = (number) => String(number).padStart(2, '0');
    const toZeroSslDate = (date) => `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}`
      + ` ${pad(date.getHours())}:${pad(date.getMinutes())}:${pad(date.getSeconds())}`;

    return new Certificate({
      id: 'c'.repeat(32),
      common_name: '1.2.3.4',
      status,
      created: toZeroSslDate(new Date(Date.now() - 24 * 60 * 60 * 1000)),
      expires: toZeroSslDate(new Date(Date.now() + 60 * 24 * 60 * 60 * 1000)),
    });
  }

  beforeEach(function beforeEach() {
    homeDir = HomeDir.createTemp();
    ({ defaultConfigs, setUp, hardResetPlatform } = createPlatformSetupConfigs(homeDir));
    getCertificate = this.sinon.stub().resolves(zeroSslCertificate('issued'));
    assertPlatformCanStart = assertPlatformCanStartFactory(
      homeDir,
      defaultConfigs,
      findReusableZeroSslCertificateFactory(
        validateZeroSslCertificateFactory(homeDir, getCertificate),
      ),
    );
  });

  afterEach(() => {
    homeDir.remove();
  });

  it('should let a set up evonode start', async () => {
    const config = setUp('testnet', { masternode: true });
    renderNodeKey(config);

    expect(await refusalOf(config)).to.be.null();
  });

  // A fullnode needs no registered identity: any key serves it.
  it('should leave fullnodes that are set up and nodes without Platform alone', async () => {
    const fullnode = setUp('testnet', { masternode: false });
    const regular = setUp('testnet', { masternode: true, evo: false });
    const coreOnly = setUp('testnet', { masternode: false, evo: false });

    expect(await refusalOf(fullnode)).to.be.null();
    expect(await refusalOf(regular)).to.be.null();
    expect(await refusalOf(coreOnly)).to.be.null();
  });

  // After `reset --platform --hard` a testnet Evo fullnode keeps Platform
  // enabled with no certificate, so `update && start` would bring up a gateway
  // that cannot serve TLS, and create the Platform volumes that setup then
  // refuses as data a reset kept.
  it('should refuse an Evo fullnode reset with --hard and point to setup --platform', async () => {
    const config = await hardResetPlatform(setUp('testnet', { masternode: false }));

    const refusal = await refusalOf(config);

    expect(refusal.problem).to.equal('its Platform part is not set up');
    expect(refusal.remedy).to.include('dashmate setup --platform --config testnet')
      .and.include('dashmate config set platform.enable false --config testnet');
  });

  // The gateway serves TLS from bundle.crt and private.key whatever the
  // provider, so a fullnode whose Platform was set up has TLS on or files of
  // its own.
  describe('a fullnode whose Platform was set up', () => {
    it('should start with a self-signed certificate', async () => {
      const config = setUp('testnet', { masternode: false });
      config.set('platform.gateway.ssl.provider', 'self-signed');

      expect(await refusalOf(config)).to.be.null();
    });

    it('should start with certificate files that dashmate does not manage', async () => {
      const config = await hardResetPlatform(setUp('testnet', { masternode: false }));
      const sslDir = homeDir.joinPath('testnet', 'platform', 'gateway', 'ssl');
      fs.mkdirSync(sslDir, { recursive: true });
      fs.writeFileSync(path.join(sslDir, 'bundle.crt'), 'certificate');
      fs.writeFileSync(path.join(sslDir, 'private.key'), 'key');

      expect(await refusalOf(config)).to.be.null();
    });

    // Rootful Docker mounts what the CLI user may not read: files the user
    // cannot look at are not proof that there are none.
    it('should start with certificate files the CLI user cannot read', async function it() {
      if (process.getuid?.() === 0) {
        this.skip();
      }

      const config = await hardResetPlatform(setUp('testnet', { masternode: false }));
      const sslDir = homeDir.joinPath('testnet', 'platform', 'gateway', 'ssl');
      const protectedDir = homeDir.joinPath('protected');
      fs.mkdirSync(sslDir, { recursive: true });
      fs.mkdirSync(protectedDir);
      fs.writeFileSync(path.join(protectedDir, 'bundle.crt'), 'certificate');
      fs.writeFileSync(path.join(protectedDir, 'private.key'), 'key');
      fs.symlinkSync(path.join(protectedDir, 'bundle.crt'), path.join(sslDir, 'bundle.crt'));
      fs.symlinkSync(path.join(protectedDir, 'private.key'), path.join(sslDir, 'private.key'));
      fs.chmodSync(protectedDir, 0o000);

      try {
        expect(await refusalOf(config)).to.be.null();
      } finally {
        fs.chmodSync(protectedDir, 0o700);
      }
    });

    // Setup cannot set these up, so it is not offered; nor is the reset that
    // led here a Platform-only setup's to repair.
    it('should leave a fullnode that is not a preset alone', async () => {
      const mainnet = await hardResetPlatform(setUp('mainnet', { masternode: false }));
      const custom = new Config('my-testnet', (await hardResetPlatform(setUp('testnet', { masternode: false }))).getStoredOptions());
      const seed = new Config('local_seed', defaultConfigs.get('local').getStoredOptions());
      seed.set('group', 'local');
      seed.set('core.masternode.enable', false);
      seed.set('platform.drive.tenderdash.mode', 'seed');

      expect(mainnet.get('platform.enable')).to.be.false();
      expect(await refusalOf(mainnet)).to.be.null();
      expect(await refusalOf(custom)).to.be.null();
      expect(await refusalOf(seed)).to.be.null();
    });
  });

  // ZeroSSL saves the answers, TLS still off, when it creates the
  // certificate, before the certificate is verified and installed.
  describe('a fullnode whose ZeroSSL setup stopped half-way', () => {
    let pair;
    let other;
    let sslDir;

    before(function before() {
      // RSA key generation is slow in pure JS.
      this.timeout(30000);
      pair = issueCertificate({ ip: '1.2.3.4' });
      other = issueCertificate({ ip: '1.2.3.4' });
    });

    beforeEach(() => {
      sslDir = homeDir.joinPath('testnet', 'platform', 'gateway', 'ssl');
    });

    /**
     * @return {Promise<Config>} a reset Evo fullnode with ZeroSSL's mid-run save
     */
    async function stoppedAfterZeroSslSave() {
      const config = await hardResetPlatform(setUp('testnet', { masternode: false }));
      config.set('platform.gateway.ssl.provider', 'zerossl');
      config.set('platform.gateway.ssl.providerConfigs.zerossl.apiKey', 'k'.repeat(32));
      config.set('platform.gateway.ssl.providerConfigs.zerossl.id', 'c'.repeat(32));

      return config;
    }

    /**
     * @param {Object} files - content by file name
     */
    function installCertificateFiles(files) {
      fs.mkdirSync(sslDir, { recursive: true });
      Object.entries(files)
        .forEach(([fileName, content]) => fs.writeFileSync(path.join(sslDir, fileName), content));
    }

    // The gateway mounts both files. Without them it serves no TLS, and the
    // mounts would leave directories where the recovery writes the files.
    it('should refuse it after a failed verification, with the commands that finish it', async () => {
      const config = await stoppedAfterZeroSslSave();

      const refusal = await refusalOf(config);

      expect(refusal.problem).to.equal('its Platform setup did not finish, and the gateway has no certificate');
      expect(refusal.remedy).to.include('dashmate ssl obtain --force --config testnet');
    });

    it('should refuse it with only one of the certificate files', async () => {
      const config = await stoppedAfterZeroSslSave();
      installCertificateFiles({ 'bundle.crt': pair.pem });

      expect((await refusalOf(config)).remedy).to.include('dashmate ssl obtain --force --config testnet');
    });

    // Compose creates a missing bind-mount source as a directory, so a start
    // before the fix left these. ssl obtain cannot write a file over them.
    it('should refuse it while directories stand where the certificate files go', async () => {
      const config = await stoppedAfterZeroSslSave();
      fs.mkdirSync(path.join(sslDir, 'bundle.crt'), { recursive: true });
      fs.mkdirSync(path.join(sslDir, 'private.key'));

      const { remedy } = await refusalOf(config);

      expect(remedy).to.include(`rmdir ${path.join(sslDir, 'bundle.crt')}`)
        .and.include(`rmdir ${path.join(sslDir, 'private.key')}`)
        .and.include('dashmate ssl obtain --force --config testnet');
      expect(remedy.indexOf('rmdir')).to.be.below(remedy.indexOf('ssl obtain'));
    });

    it('should refuse it with links that point nowhere', async () => {
      const config = await stoppedAfterZeroSslSave();
      fs.mkdirSync(sslDir, { recursive: true });
      fs.symlinkSync(path.join(sslDir, 'gone.crt'), path.join(sslDir, 'bundle.crt'));
      fs.symlinkSync(path.join(sslDir, 'gone.key'), path.join(sslDir, 'private.key'));

      expect((await refusalOf(config)).remedy).to.include('dashmate ssl obtain --force --config testnet');
    });

    // The gateway cannot serve a pair that does not load or match.
    it('should refuse it with a pair the gateway cannot serve', async () => {
      const config = await stoppedAfterZeroSslSave();
      installCertificateFiles({ 'bundle.crt': pair.pem, 'private.key': other.keyPem });

      expect((await refusalOf(config)).problem).to.equal('its Platform setup did not finish, and the gateway has no certificate');

      installCertificateFiles({ 'bundle.crt': '', 'private.key': '' });

      expect(await refusalOf(config)).to.not.be.null();
    });

    // A run that stopped while writing the bundle, out of disk space say,
    // leaves part of it next to an intact key. ZeroSSL issued the
    // certificate, and a plain obtain downloads it again.
    it('should refuse it with a truncated bundle, and install the issued certificate again', async () => {
      const config = await stoppedAfterZeroSslSave();
      installCertificateFiles({
        'bundle.crt': pair.pem.slice(0, pair.pem.length / 2),
        'private.key': pair.keyPem,
      });

      const refusal = await refusalOf(config);

      expect(refusal.problem).to.equal('its Platform setup did not finish, and the gateway has no certificate');
      expect(refusal.remedy).to.include('ZeroSSL issued its certificate')
        .and.include('dashmate ssl obtain --config testnet')
        .and.not.include('--force');
      expect(getCertificate).to.have.been.calledWith('k'.repeat(32), 'c'.repeat(32));

      const pending = await assertPlatformCanStart.findPendingSetup(config);

      expect(pending.canStart).to.be.undefined();
      expect(pending.remedy).to.equal(refusal.remedy);
    });

    ['cancelled', 'expired'].forEach((status) => {
      it(`should advise a new certificate when ZeroSSL reports the saved one ${status}`, async () => {
        getCertificate.resolves(zeroSslCertificate(status));
        const config = await stoppedAfterZeroSslSave();
        installCertificateFiles({
          'bundle.crt': pair.pem.slice(0, pair.pem.length / 2),
          'private.key': pair.keyPem,
        });

        expect((await refusalOf(config)).remedy)
          .to.include('dashmate ssl obtain --force --config testnet')
          .and.include(`ZeroSSL reports it ${status}`);
      });
    });

    // A plain obtain decides by the same validation once ZeroSSL answers.
    it('should advise a plain obtain when ZeroSSL cannot be asked', async () => {
      getCertificate.rejects(new ProviderUnreachableError('fetch failed'));
      const config = await stoppedAfterZeroSslSave();
      installCertificateFiles({
        'bundle.crt': pair.pem.slice(0, pair.pem.length / 2),
        'private.key': pair.keyPem,
      });

      expect((await refusalOf(config)).remedy)
        .to.include('could not ask ZeroSSL')
        .and.include('fetch failed')
        .and.include('dashmate ssl obtain --config testnet')
        .and.not.include('--force');
    });

    it('should not ask ZeroSSL while the gateway can serve the installed certificate', async () => {
      const config = await stoppedAfterZeroSslSave();
      installCertificateFiles({ 'bundle.crt': pair.pem, 'private.key': pair.keyPem });

      expect(await refusalOf(config)).to.be.null();
      expect(getCertificate).to.not.have.been.called();
    });

    // Clients refuse an expired certificate, so the gateway serves no TLS.
    it('should refuse it with an expired certificate', async function it() {
      this.timeout(30000);
      const expired = issueCertificate({ ip: '1.2.3.4', days: -1 });
      const config = await stoppedAfterZeroSslSave();
      installCertificateFiles({ 'bundle.crt': expired.pem, 'private.key': expired.keyPem });

      expect((await refusalOf(config)).problem).to.equal('its Platform setup did not finish, and the gateway has no certificate');
    });

    // TLS works from the installed pair: only renewal is not managed yet,
    // which update reports.
    it('should start once the issued certificate is installed', async () => {
      const config = await stoppedAfterZeroSslSave();
      installCertificateFiles({ 'bundle.crt': pair.pem, 'private.key': pair.keyPem });

      expect(await refusalOf(config)).to.be.null();
      const pending = await assertPlatformCanStart.findPendingSetup(config);
      expect(pending.canStart).to.be.true();
      expect(pending.remedy).to.include('dashmate ssl obtain --config testnet');
    });

    // Rootful Docker mounts what the CLI user may not read: files the user
    // cannot look at are not proof that they do not work.
    it('should start with certificate files the CLI user cannot read', async function it() {
      if (process.getuid?.() === 0) {
        this.skip();
      }

      const config = await stoppedAfterZeroSslSave();
      const protectedDir = homeDir.joinPath('protected');
      fs.mkdirSync(sslDir, { recursive: true });
      fs.mkdirSync(protectedDir);
      fs.writeFileSync(path.join(protectedDir, 'bundle.crt'), pair.pem);
      fs.writeFileSync(path.join(protectedDir, 'private.key'), pair.keyPem);
      fs.symlinkSync(path.join(protectedDir, 'bundle.crt'), path.join(sslDir, 'bundle.crt'));
      fs.symlinkSync(path.join(protectedDir, 'private.key'), path.join(sslDir, 'private.key'));
      fs.chmodSync(protectedDir, 0o000);

      try {
        expect(await refusalOf(config)).to.be.null();
      } finally {
        fs.chmodSync(protectedDir, 0o700);
      }
    });

    it('should start once the setup is finished', async () => {
      const config = await stoppedAfterZeroSslSave();
      installCertificateFiles({ 'bundle.crt': pair.pem, 'private.key': pair.keyPem });
      config.set('platform.gateway.ssl.enabled', true);

      expect(await refusalOf(config)).to.be.null();
      expect(await assertPlatformCanStart.findPendingSetup(config)).to.be.null();
    });
  });

  // After a testnet hard reset Platform stays enabled and the key is gone.
  // Tenderdash would start on a node_key.json that says "null".
  it('should refuse an evonode reset with --hard and point to setup --platform', async () => {
    const config = await hardResetPlatform(setUp('testnet', { masternode: true }));
    renderNodeKey(config);

    const refusal = await refusalOf(config);

    expect(refusal.message).to.include('dashmate setup --platform --config testnet');
  });

  it('should refuse a masternode whose Tenderdash does not validate', async () => {
    const config = setUp('testnet', { masternode: true });
    config.set('platform.drive.tenderdash.mode', 'full');
    renderNodeKey(config);

    expect((await refusalOf(config)).message)
      .to.include('dashmate config set platform.drive.tenderdash.mode validator --config testnet');
  });

  // An untouched testnet preset enables Platform on a masternode by default.
  it('should point a preset that was never set up to full setup', async () => {
    expect((await refusalOf(defaultConfigs.get('testnet'))).message).to.include('dashmate setup testnet');
  });

  // config.json is authoritative and written first, so a run killed before
  // rendering leaves Tenderdash an older identity.
  it('should refuse while the rendered node key is not the configured one', async () => {
    const config = setUp('testnet', { masternode: true });
    const other = generateTenderdashNodeKey();
    renderNodeKey(config, { id: deriveTenderdashNodeId(other), key: other });

    expect((await refusalOf(config)).message).to.include('dashmate config render --config testnet');
  });

  it('should refuse while no node key has been rendered', async () => {
    const config = setUp('testnet', { masternode: true });

    expect((await refusalOf(config)).message).to.include('dashmate config render --config testnet');
  });

  it('should give the finishing commands of an interrupted ZeroSSL setup', async () => {
    const config = setUp('testnet', { masternode: true });
    config.set('platform.gateway.ssl.enabled', false);
    config.set('platform.gateway.ssl.provider', 'zerossl');
    config.set('platform.gateway.ssl.providerConfigs.zerossl.apiKey', 'k'.repeat(32));
    config.set('platform.gateway.ssl.providerConfigs.zerossl.id', 'c'.repeat(32));
    renderNodeKey(config, { id: null, key: null });

    expect((await refusalOf(config)).message).to.include('dashmate ssl obtain --force --config testnet');
  });

  // The local group's members are masternodes set up by `setup local`, which
  // registers their keys itself.
  it('should let a local group member start', async () => {
    const options = defaultConfigs.get('local').getStoredOptions();
    const config = new Config('local_1', options);
    const key = generateTenderdashNodeKey();
    config.set('group', 'local');
    config.set('core.masternode.operator.privateKey', '1bbac8e471b80050ea44afb514c87cb1ef03db5c66e643c7d6e0f09b29bedbc0');
    config.set('externalIp', '127.0.0.1');
    config.set('platform.drive.tenderdash.mode', 'validator');
    config.set('platform.drive.tenderdash.node.key', key);
    config.set('platform.drive.tenderdash.node.id', deriveTenderdashNodeId(key));
    renderNodeKey(config);

    expect(await refusalOf(config)).to.be.null();
  });

  /**
   * @return {Config} local_1 of the local group, without a node key
   */
  function keylessLocalMember() {
    const config = new Config('local_1', defaultConfigs.get('local').getStoredOptions());
    config.set('group', 'local');
    config.set('core.masternode.operator.privateKey', '1bbac8e471b80050ea44afb514c87cb1ef03db5c66e643c7d6e0f09b29bedbc0');
    config.set('externalIp', '127.0.0.1');

    return config;
  }

  // The local group's reset removes every config whose group is local, and
  // `setup local` builds its configs again by name, wherever they are now: a
  // member moved to another group would be taken back and given a new key.
  // Neither is advised; the remedy is the setting itself, as for every other
  // masternode.
  it('should name the settings to fix for a local group member without a key, and advise no reset or setup', async () => {
    const config = keylessLocalMember();

    const refusal = await refusalOf(config);

    expect(refusal.problem).to.equal('this masternode has no Platform node key');
    expect(refusal.remedy).to.include('platform.drive.tenderdash.node.key')
      .and.include('platform.drive.tenderdash.node.id')
      .and.include(homeDir.joinPath('config.json'))
      .and.include('dashmate config render --config local_1')
      .and.include('dashmate config set platform.enable false --config local_1');
    expect(refusal.remedy).to.not.include('reset --')
      .and.not.include('dashmate setup')
      .and.not.include('config set platform.drive.tenderdash.node');
  });

  // A hard reset removes a member of a group setup does not build, or a
  // config that is not a preset, entirely, and setup cannot create it again:
  // the remedy is the setting itself.
  describe('a masternode without a key that setup cannot set up again', () => {
    /**
     * @param {string} name
     * @param {string|null} group
     * @return {Config}
     */
    async function keyless(name, group) {
      const reset = await hardResetPlatform(setUp('testnet', { masternode: true }));
      const config = new Config(name, reset.getStoredOptions());
      config.set('group', group);

      return config;
    }

    [
      ['a member of a custom group', 'testnet_1', 'production'],
      ['a config that is not a preset', 'evo-1', null],
      // `setup local` builds only local_1 to local_N and local_seed.
      ['a config of the local group that setup local does not build', 'custom', 'local'],
    ].forEach(([description, name, group]) => {
      it(`should name the setting to fix for ${description}, and advise no reset`, async () => {
        const config = await keyless(name, group);

        const refusal = await refusalOf(config);

        expect(refusal.problem).to.equal('this masternode has no Platform node key');
        expect(refusal.remedy).to.include('platform.drive.tenderdash.node.key')
          .and.include('platform.drive.tenderdash.node.id')
          .and.include(homeDir.joinPath('config.json'))
          .and.include(`dashmate config render --config ${name}`)
          .and.include(`dashmate config set platform.enable false --config ${name}`);
        expect(refusal.remedy).to.not.include('reset --')
          .and.not.include('dashmate setup')
          .and.not.include('config set platform.drive.tenderdash.node');
      });
    });
  });

  // update words the refusal itself.
  it('should carry the problem and the remedy apart', async () => {
    const config = await hardResetPlatform(setUp('testnet', { masternode: true }));

    const refusal = await refusalOf(config);

    expect(refusal.problem).to.equal('this masternode has no Platform node key');
    expect(refusal.remedy).to.include('dashmate setup --platform --config testnet')
      .and.include('dashmate config set platform.enable false --config testnet');
  });

  // Tenderdash derives its ID from the key, so the id field alone proves
  // nothing.
  it('should refuse a rendered key of another node behind the right id field', async () => {
    const config = setUp('testnet', { masternode: true });
    renderNodeKey(config, {
      id: config.get('platform.drive.tenderdash.node.id'),
      key: generateTenderdashNodeKey(),
    });

    expect((await refusalOf(config)).message).to.include('dashmate config render --config testnet');
  });

  // ssl obtain cannot write a certificate file over a directory that a
  // start of Compose left in its place.
  it('should tell a masternode to remove directories where the certificate files go', async () => {
    const config = setUp('testnet', { masternode: true });
    config.set('platform.gateway.ssl.enabled', false);
    config.set('platform.gateway.ssl.provider', 'zerossl');
    config.set('platform.gateway.ssl.providerConfigs.zerossl.apiKey', 'k'.repeat(32));
    config.set('platform.gateway.ssl.providerConfigs.zerossl.id', 'c'.repeat(32));
    renderNodeKey(config);
    const sslDir = homeDir.joinPath('testnet', 'platform', 'gateway', 'ssl');
    fs.mkdirSync(path.join(sslDir, 'bundle.crt'), { recursive: true });
    fs.mkdirSync(path.join(sslDir, 'private.key'));

    const { remedy } = await refusalOf(config);

    expect(remedy).to.include(`rmdir ${path.join(sslDir, 'bundle.crt')}`)
      .and.include('dashmate ssl obtain --force --config testnet');
  });

  // `config render` writes the saved identity, but TLS stays off.
  it('should refuse an interrupted ZeroSSL setup even once its files are rendered', async () => {
    const config = setUp('testnet', { masternode: true });
    config.set('platform.gateway.ssl.enabled', false);
    config.set('platform.gateway.ssl.provider', 'zerossl');
    config.set('platform.gateway.ssl.providerConfigs.zerossl.apiKey', 'k'.repeat(32));
    config.set('platform.gateway.ssl.providerConfigs.zerossl.id', 'c'.repeat(32));
    renderNodeKey(config);

    expect((await refusalOf(config)).message).to.include('dashmate ssl obtain --force --config testnet');
  });

  describe('a setup an update has to wait for', () => {
    // A certificate obtained by update would make Platform setup refuse.
    it('should find an Evo fullnode reset with --hard', async () => {
      const config = await hardResetPlatform(setUp('testnet', { masternode: false }));

      expect((await assertPlatformCanStart.findPendingSetup(config)).remedy)
        .to.include('dashmate setup --platform --config testnet');
    });

    it('should find what keeps a masternode from starting', async () => {
      const config = await hardResetPlatform(setUp('testnet', { masternode: true }));

      expect((await assertPlatformCanStart.findPendingSetup(config)).problem)
        .to.equal('this masternode has no Platform node key');
    });

    it('should find nothing on a node that is set up, or not a preset', async () => {
      const config = setUp('testnet', { masternode: false });
      const other = new Config('local_seed', defaultConfigs.get('local').getStoredOptions());
      other.set('core.masternode.enable', false);
      other.set('platform.drive.tenderdash.mode', 'seed');

      expect(await assertPlatformCanStart.findPendingSetup(config)).to.be.null();
      expect(await assertPlatformCanStart.findPendingSetup(other)).to.be.null();
    });
  });

  it('should never print the node key', async () => {
    const config = setUp('testnet', { masternode: true });
    config.set('platform.drive.tenderdash.mode', 'full');
    renderNodeKey(config);

    expect((await refusalOf(config)).message)
      .to.not.include(config.get('platform.drive.tenderdash.node.key').slice(0, 16));
  });
});
