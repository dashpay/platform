import getBaseConfigFactory from '../../../configs/defaults/getBaseConfigFactory.js';
import renderGatewayRestartCommand from '../../../src/ssl/renderGatewayRestartCommand.js';

describe('renderGatewayRestartCommand', () => {
  let config;

  beforeEach(() => {
    config = getBaseConfigFactory()();
  });

  it('should restart Platform alone', () => {
    expect(renderGatewayRestartCommand(config)).to.equal('dashmate restart --config base --platform');
  });

  // stop --platform stops Core too when the quorum list is enabled, without
  // the DKG check a masternode gets before Core stops, and starting Platform
  // alone then fails. A whole-node restart keeps that check.
  it('should restart the whole node when the quorum list is enabled', () => {
    config.set('platform.quorumList.enabled', true);

    expect(renderGatewayRestartCommand(config)).to.equal('dashmate restart --config base');
  });
});
