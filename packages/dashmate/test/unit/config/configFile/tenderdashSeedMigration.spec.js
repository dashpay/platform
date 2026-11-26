import getBaseConfigFactory from '../../../../configs/defaults/getBaseConfigFactory.js';
import getConfigFileMigrationsFactory from '../../../../configs/getConfigFileMigrationsFactory.js';

describe('Tenderdash seed migrations', () => {
  const seedsPath = 'platform.drive.tenderdash.p2p.seeds';
  const self = { id: 'a'.repeat(40), host: '192.0.2.1', port: 26656 };
  const peer = { id: 'b'.repeat(40), host: '192.0.2.2', port: 26656 };

  for (const [version, network] of [
    ['0.25.0-dev.30', 'testnet'],
    ['1.0.0', 'mainnet'],
    ['1.1.0-dev.1', 'mainnet'],
    ['1.5.0', 'mainnet'],
    ['1.5.0', 'testnet'],
  ]) {
    it(`should exclude the local node from ${network} defaults in migration ${version}`, () => {
      const getBaseConfig = getBaseConfigFactory();
      const base = getBaseConfig();
      const networkDefaults = getBaseConfig();
      networkDefaults.set(seedsPath, [self, peer]);
      const defaults = new Map([['base', base], [network, networkDefaults]]);
      const migrations = getConfigFileMigrationsFactory(null, defaults)();
      const options = base.getStoredOptions();
      options.network = network;
      options.platform.drive.tenderdash.node.id = self.id;

      migrations[version]({ configs: { example: options } });

      expect(options.platform.drive.tenderdash.p2p.seeds).to.deep.equal([peer]);
      expect(networkDefaults.getStored(seedsPath)).to.deep.equal([self, peer]);
    });
  }
});
