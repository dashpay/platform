import { expect } from 'chai';
import HomeDir from '../../../src/config/HomeDir.js';
import resolveConfigDirectory, {
  assertSafeConfigName,
} from '../../../src/config/resolve-config-directory.js';

describe('resolveConfigDirectory', () => {
  const homeDir = new HomeDir('/tmp/dashmate-test-home', true);

  it('resolves a safe name as a direct child', () => {
    expect(resolveConfigDirectory(homeDir, 'testnet-1'))
      .to.equal('/tmp/dashmate-test-home/testnet-1');
  });

  it('rejects unsafe names', () => {
    for (const name of ['', '.', '..', '../mainnet', 'slot/../mainnet', 'a/b', 'a\\b']) {
      expect(() => assertSafeConfigName(name), name).to.throw('path-safe segment');
    }
  });

  it('rejects names reserved for repository state', () => {
    for (const name of [
      'config.json',
      'config.json.lock',
      'config.json.rescue',
      'config.json.render-pending',
      'CONFIG.JSON',
      'Config.Json.Lock',
      'CONFIG.JSON.RESCUE',
      'Config.Json.Render-Pending',
      'config.json.',
      'CONFIG.JSON.LOCK...',
    ]) {
      expect(() => assertSafeConfigName(name), name).to.throw('reserved by Dashmate');
    }
  });
});
