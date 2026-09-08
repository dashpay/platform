import { expect } from 'chai';
import HomeDir from '../../../src/config/HomeDir.js';
import resolveConfigDirectory, {
  assertSafeConfigName,
  assertConfigNameAvailable,
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
      'CONFIG.JSON',
      'config.json.',
      'Config.Json...',
    ]) {
      expect(() => assertConfigNameAvailable(name), name).to.throw('reserved by Dashmate');
    }
  });

  // Everything else the repository writes beside the config file is
  // dot-prefixed, and a config name cannot begin with a period - so these
  // collide with nothing and refusing them would only break existing nodes.
  it('accepts names that merely resemble repository state', () => {
    for (const name of [
      'config.json.lock',
      'config.json.rescue',
      'config.json.render-pending',
    ]) {
      expect(() => assertConfigNameAvailable(name), name).to.not.throw();
    }
  });
});
