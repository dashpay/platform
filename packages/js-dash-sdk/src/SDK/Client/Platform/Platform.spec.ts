import { expect } from 'chai';
import { getLatestProtocolVersion } from '@dashevo/wasm-dpp';
import { Platform } from './index';
import 'mocha';
import Client from '../Client';

describe('Dash - Platform', () => {
  it('should provide expected class', () => {
    expect(Platform.name).to.be.equal('Platform');
    expect(Platform.constructor.name).to.be.equal('Function');
  });

  it('should use the protocol version passed through options', async () => {
    const platform = new Platform({
      client: new Client({ network: 'testnet' }),
      network: 'testnet',
      driveProtocolVersion: 1,
    });

    await platform.initialize();

    expect(platform.protocolVersion).to.equal(1);
  });

  it('should default to the latest protocol version on testnet', async () => {
    // Regression: testnet must not be pinned to an old protocol version.
    // wasm-dpp deserializes fetched contracts at this version, and an old
    // version downgrades a V1 config (sized_integer_types) to V0, which the
    // network rejects on contract update ("config version 0 is not supported").
    const platform = new Platform({
      client: new Client({ network: 'testnet' }),
      network: 'testnet',
    });

    await platform.initialize();

    expect(platform.protocolVersion).to.equal(getLatestProtocolVersion());
  });
});
