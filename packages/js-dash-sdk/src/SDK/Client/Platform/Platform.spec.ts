import { expect } from 'chai';
import { Platform } from "./index";
import 'mocha';
import Client from '../Client';
import { latestVersion as latestProtocolVersion } from "@dashevo/dpp/lib/protocolVersion";

describe('Dash - Platform', () => {

  it('should provide expected class', function () {
    expect(Platform.name).to.be.equal('Platform')
    expect(Platform.constructor.name).to.be.equal('Function')
  });

  it('should set protocol version for DPP though options', () => {
    const platform = new Platform({
      client: new Client(),
      network: 'testnet',
      driveProtocolVersion: 42,
    });

    expect(platform.dpp.protocolVersion).to.equal(42);
  });

  it('should set protocol version for DPP using mapping', () => {
    const platform = new Platform({
      client: new Client(),
      network: 'testnet',
    });

    expect(platform.dpp.protocolVersion).to.equal(0);
  });

  it('should set protocol version for DPP using latest version', () => {
    const platform = new Platform({
      client: new Client(),
      network: 'unknown',
    });

    expect(platform.dpp.protocolVersion).to.equal(latestProtocolVersion);
  });
});
