import { expect } from 'chai';
import HomeDir from '../../../src/config/HomeDir.js';
import resolveConfigDirectory, {
  assertSafeConfigName,
} from '../../../src/config/resolveConfigDirectory.js';

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
});
