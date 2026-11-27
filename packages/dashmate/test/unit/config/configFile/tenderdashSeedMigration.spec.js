import getBaseConfigFactory from '../../../../configs/defaults/getBaseConfigFactory.js';
import getConfigFileMigrationsFactory from '../../../../configs/getConfigFileMigrationsFactory.js';
import deriveTenderdashNodeId from '../../../../src/tenderdash/deriveTenderdashNodeId.js';
import generateTenderdashNodeKey from '../../../../src/tenderdash/generateTenderdashNodeKey.js';

describe('Tenderdash seed migrations', () => {
  const seedsPath = 'platform.drive.tenderdash.p2p.seeds';
  const key = generateTenderdashNodeKey();
  const self = { id: deriveTenderdashNodeId(key), host: '192.0.2.1', port: 26656 };
  const peer = { id: 'b'.repeat(40), host: '192.0.2.2', port: 26656 };

  for (const [version, network] of [
    ['0.25.0-dev.30', 'testnet'],
    ['1.0.0', 'mainnet'],
    ['1.1.0-dev.1', 'mainnet'],
    ['1.5.0', 'mainnet'],
    ['1.5.0', 'testnet'],
  ]) {
    it(`should keep the full ${network} default seed list in migration ${version}`, () => {
      const getBaseConfig = getBaseConfigFactory();
      const base = getBaseConfig();
      const networkDefaults = getBaseConfig();
      networkDefaults.set(seedsPath, [self, peer]);
      const defaults = new Map([['base', base], [network, networkDefaults]]);
      const migrations = getConfigFileMigrationsFactory(null, defaults)();
      const options = base.getStoredOptions();
      options.network = network;
      const { tenderdash } = options.platform.drive;
      tenderdash.node = { id: peer.id, key };

      migrations[version]({ configs: { example: options } });

      expect(tenderdash.p2p.seeds).to.deep.equal([self, peer]);
      expect(networkDefaults.getStored(seedsPath)).to.deep.equal([self, peer]);
    });
  }

  it('should preserve all other default P2P fields in the 1.0.0 migration', () => {
    const getBaseConfig = getBaseConfigFactory();
    const base = getBaseConfig();
    const mainnet = getBaseConfig();
    const p2pPath = 'platform.drive.tenderdash.p2p';
    mainnet.set(`${p2pPath}.port`, 27656);
    mainnet.set(`${p2pPath}.persistentPeers`, [peer]);
    mainnet.set(seedsPath, [self, peer]);
    const defaults = new Map([['base', base], ['mainnet', mainnet]]);
    const migrations = getConfigFileMigrationsFactory(null, defaults)();
    const options = base.getStoredOptions();
    options.network = 'mainnet';
    const { tenderdash } = options.platform.drive;
    tenderdash.node.id = self.id;

    migrations['1.0.0']({ configs: { example: options } });

    expect(tenderdash.p2p).to.deep.equal(mainnet.getStored(p2pPath));
  });
});
