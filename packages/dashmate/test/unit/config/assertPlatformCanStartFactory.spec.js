import fs from 'fs';
import path from 'path';
import HomeDir from '../../../src/config/HomeDir.js';
import Config from '../../../src/config/Config.js';
import assertPlatformCanStartFactory from '../../../src/config/assertPlatformCanStartFactory.js';
import generateTenderdashNodeKey from '../../../src/tenderdash/generateTenderdashNodeKey.js';
import deriveTenderdashNodeId from '../../../src/tenderdash/deriveTenderdashNodeId.js';
import createPlatformSetupConfigs from '../../fixtures/platformSetupConfigs.js';

describe('assertPlatformCanStart', () => {
  let homeDir;
  let defaultConfigs;
  let setUp;
  let hardResetPlatform;
  let assertPlatformCanStart;

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
   * @return {Error|null}
   */
  function refusalOf(config) {
    try {
      assertPlatformCanStart(config);
    } catch (e) {
      return e;
    }

    return null;
  }

  beforeEach(() => {
    homeDir = HomeDir.createTemp();
    ({ defaultConfigs, setUp, hardResetPlatform } = createPlatformSetupConfigs(homeDir));
    assertPlatformCanStart = assertPlatformCanStartFactory(homeDir, defaultConfigs);
  });

  afterEach(() => {
    homeDir.remove();
  });

  it('should let a set up evonode start', () => {
    const config = setUp('testnet', { masternode: true });
    renderNodeKey(config);

    expect(refusalOf(config)).to.be.null();
  });

  // A fullnode needs no registered identity: any key serves it.
  it('should leave fullnodes that are set up and nodes without Platform alone', () => {
    const fullnode = setUp('testnet', { masternode: false });
    const regular = setUp('testnet', { masternode: true, evo: false });
    const coreOnly = setUp('testnet', { masternode: false, evo: false });

    expect(refusalOf(fullnode)).to.be.null();
    expect(refusalOf(regular)).to.be.null();
    expect(refusalOf(coreOnly)).to.be.null();
  });

  // After `reset --platform --hard` a testnet Evo fullnode keeps Platform
  // enabled with no certificate, so `update && start` would bring up a gateway
  // that cannot serve TLS, and create the Platform volumes that setup then
  // refuses as data a reset kept.
  it('should refuse an Evo fullnode reset with --hard and point to setup --platform', async () => {
    const config = await hardResetPlatform(setUp('testnet', { masternode: false }));

    const refusal = refusalOf(config);

    expect(refusal.problem).to.equal('its Platform part is not set up');
    expect(refusal.remedy).to.include('dashmate setup --platform --config testnet')
      .and.include('dashmate config set platform.enable false --config testnet');
  });

  // The gateway serves TLS from bundle.crt and private.key whatever the
  // provider, so a fullnode whose Platform was set up has TLS on or files of
  // its own.
  describe('a fullnode whose Platform was set up', () => {
    it('should start with a self-signed certificate', () => {
      const config = setUp('testnet', { masternode: false });
      config.set('platform.gateway.ssl.provider', 'self-signed');

      expect(refusalOf(config)).to.be.null();
    });

    it('should start with certificate files that dashmate does not manage', async () => {
      const config = await hardResetPlatform(setUp('testnet', { masternode: false }));
      const sslDir = homeDir.joinPath('testnet', 'platform', 'gateway', 'ssl');
      fs.mkdirSync(sslDir, { recursive: true });
      fs.writeFileSync(path.join(sslDir, 'bundle.crt'), 'certificate');
      fs.writeFileSync(path.join(sslDir, 'private.key'), 'key');

      expect(refusalOf(config)).to.be.null();
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
        expect(refusalOf(config)).to.be.null();
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
      expect(refusalOf(mainnet)).to.be.null();
      expect(refusalOf(custom)).to.be.null();
      expect(refusalOf(seed)).to.be.null();
    });
  });

  // After a testnet hard reset Platform stays enabled and the key is gone.
  // Tenderdash would start on a node_key.json that says "null".
  it('should refuse an evonode reset with --hard and point to setup --platform', async () => {
    const config = await hardResetPlatform(setUp('testnet', { masternode: true }));
    renderNodeKey(config);

    const refusal = refusalOf(config);

    expect(refusal.message).to.include('dashmate setup --platform --config testnet');
  });

  it('should refuse a masternode whose Tenderdash does not validate', () => {
    const config = setUp('testnet', { masternode: true });
    config.set('platform.drive.tenderdash.mode', 'full');
    renderNodeKey(config);

    expect(refusalOf(config).message)
      .to.include('dashmate config set platform.drive.tenderdash.mode validator --config testnet');
  });

  // An untouched testnet preset enables Platform on a masternode by default.
  it('should point a preset that was never set up to full setup', () => {
    expect(refusalOf(defaultConfigs.get('testnet')).message).to.include('dashmate setup testnet');
  });

  // config.json is authoritative and written first, so a run killed before
  // rendering leaves Tenderdash an older identity.
  it('should refuse while the rendered node key is not the configured one', () => {
    const config = setUp('testnet', { masternode: true });
    const other = generateTenderdashNodeKey();
    renderNodeKey(config, { id: deriveTenderdashNodeId(other), key: other });

    expect(refusalOf(config).message).to.include('dashmate config render --config testnet');
  });

  it('should refuse while no node key has been rendered', () => {
    const config = setUp('testnet', { masternode: true });

    expect(refusalOf(config).message).to.include('dashmate config render --config testnet');
  });

  it('should give the finishing commands of an interrupted ZeroSSL setup', () => {
    const config = setUp('testnet', { masternode: true });
    config.set('platform.gateway.ssl.enabled', false);
    config.set('platform.gateway.ssl.provider', 'zerossl');
    config.set('platform.gateway.ssl.providerConfigs.zerossl.apiKey', 'k'.repeat(32));
    config.set('platform.gateway.ssl.providerConfigs.zerossl.id', 'c'.repeat(32));
    renderNodeKey(config, { id: null, key: null });

    expect(refusalOf(config).message).to.include('dashmate ssl obtain --force --config testnet');
  });

  // The local group's members are masternodes set up by `setup local`, which
  // registers their keys itself.
  it('should let a local group member start', () => {
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

    expect(refusalOf(config)).to.be.null();
  });

  // Platform-only setup does not support a group, whose identities belong to
  // the whole group.
  it('should send a local group member without a key to the group setup', () => {
    const config = new Config('local_1', defaultConfigs.get('local').getStoredOptions());
    config.set('group', 'local');
    config.set('core.masternode.operator.privateKey', '1bbac8e471b80050ea44afb514c87cb1ef03db5c66e643c7d6e0f09b29bedbc0');
    config.set('externalIp', '127.0.0.1');

    const refusal = refusalOf(config);

    expect(refusal.message).to.include('dashmate group reset --group local --hard')
      .and.include('dashmate setup local');
    expect(refusal.message).to.not.include('setup --platform');
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

        const refusal = refusalOf(config);

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

    const refusal = refusalOf(config);

    expect(refusal.problem).to.equal('this masternode has no Platform node key');
    expect(refusal.remedy).to.include('dashmate setup --platform --config testnet')
      .and.include('dashmate config set platform.enable false --config testnet');
  });

  // Tenderdash derives its ID from the key, so the id field alone proves
  // nothing.
  it('should refuse a rendered key of another node behind the right id field', () => {
    const config = setUp('testnet', { masternode: true });
    renderNodeKey(config, {
      id: config.get('platform.drive.tenderdash.node.id'),
      key: generateTenderdashNodeKey(),
    });

    expect(refusalOf(config).message).to.include('dashmate config render --config testnet');
  });

  // `config render` writes the saved identity, but TLS stays off.
  it('should refuse an interrupted ZeroSSL setup even once its files are rendered', () => {
    const config = setUp('testnet', { masternode: true });
    config.set('platform.gateway.ssl.enabled', false);
    config.set('platform.gateway.ssl.provider', 'zerossl');
    config.set('platform.gateway.ssl.providerConfigs.zerossl.apiKey', 'k'.repeat(32));
    config.set('platform.gateway.ssl.providerConfigs.zerossl.id', 'c'.repeat(32));
    renderNodeKey(config);

    expect(refusalOf(config).message).to.include('dashmate ssl obtain --force --config testnet');
  });

  describe('a setup an update has to wait for', () => {
    // A certificate obtained by update would make Platform setup refuse.
    it('should find an Evo fullnode reset with --hard', async () => {
      const config = await hardResetPlatform(setUp('testnet', { masternode: false }));

      expect(assertPlatformCanStart.findPendingSetup(config).remedy)
        .to.include('dashmate setup --platform --config testnet');
    });

    it('should find what keeps a masternode from starting', async () => {
      const config = await hardResetPlatform(setUp('testnet', { masternode: true }));

      expect(assertPlatformCanStart.findPendingSetup(config).problem)
        .to.equal('this masternode has no Platform node key');
    });

    it('should find nothing on a node that is set up, or not a preset', () => {
      const config = setUp('testnet', { masternode: false });
      const other = new Config('local_seed', defaultConfigs.get('local').getStoredOptions());
      other.set('core.masternode.enable', false);
      other.set('platform.drive.tenderdash.mode', 'seed');

      expect(assertPlatformCanStart.findPendingSetup(config)).to.be.null();
      expect(assertPlatformCanStart.findPendingSetup(other)).to.be.null();
    });
  });

  it('should never print the node key', () => {
    const config = setUp('testnet', { masternode: true });
    config.set('platform.drive.tenderdash.mode', 'full');
    renderNodeKey(config);

    expect(refusalOf(config).message)
      .to.not.include(config.get('platform.drive.tenderdash.node.key').slice(0, 16));
  });
});
