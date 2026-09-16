import { expect } from 'chai';
import { createHelperCommandArgs } from '../../../../src/helper/api/createHttpApiServerFactory.js';

describe('createHttpApiServerFactory', () => {
  describe('createHelperCommandArgs', () => {
    it('creates arguments for a JSON status request', () => {
      expect(createHelperCommandArgs('status', {
        format: 'json',
        config: 'testnet-1',
      })).to.deep.equal([
        'status',
        '--format=json',
        '--config=testnet-1',
      ]);
    });

    it('does not route commands other than status', () => {
      expect(createHelperCommandArgs('config get', {})).to.equal(null);
      expect(createHelperCommandArgs('reset', {})).to.equal(null);
    });

    it('rejects positional and extra parameters', () => {
      expect(() => createHelperCommandArgs('status', ['--format=json']))
        .to.throw('Status parameters must be an object');
      expect(() => createHelperCommandArgs('status', {
        format: 'json',
        config: 'testnet',
        verbose: true,
      })).to.throw('Unsupported status parameter');
    });

    it('requires the JSON output format', () => {
      expect(() => createHelperCommandArgs('status', {
        format: 'plain',
        config: 'testnet',
      })).to.throw('Status format must be json');
    });

    it('rejects unsafe config names', () => {
      for (const config of ['', '../mainnet', 'name/child', 'name\nnext']) {
        expect(() => createHelperCommandArgs('status', {
          format: 'json',
          config,
        })).to.throw('Invalid config name');
      }
    });
  });
});
