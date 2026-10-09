import { expect } from 'chai';
import HomeDir from '../../../src/config/HomeDir.js';
import getBaseConfigFactory from '../../../configs/defaults/getBaseConfigFactory.js';
import renderTemplateFactory from '../../../src/templates/renderTemplateFactory.js';
import renderServiceTemplatesFactory from '../../../src/templates/renderServiceTemplatesFactory.js';

describe('Tor templates', () => {
  let config;
  let renderServiceTemplates;

  beforeEach(() => {
    config = getBaseConfigFactory(HomeDir.createTemp())();
    renderServiceTemplates = renderServiceTemplatesFactory(renderTemplateFactory());
  });

  [
    ['mainnet', null],
    ['testnet', '[test]'],
    ['devnet', '[devnet]'],
    ['local', '[regtest]'],
  ].forEach(([network, section]) => {
    it(`should provide the dedicated onion bind required by Core 24 on ${network}`, () => {
      config.set('network', network);
      config.set('core.tor.enabled', true);
      // The onion target is independent of an operator's clearnet P2P port.
      config.set('core.p2p.port', 21000);

      const dashConf = renderServiceTemplates(config)['core/dash.conf'];
      // -bind is network-only, so non-mainnet binds must be in their network section.
      const networkConf = section ? dashConf.split(section)[1] : dashConf;

      expect(dashConf).to.match(/^listenonion=1$/m);
      expect(networkConf.match(/^bind=.*$/gm)).to.deep.equal([
        'bind=0.0.0.0',
        'bind=127.0.0.1=onion',
      ]);
      if (network === 'mainnet') {
        const beforeOnionBind = dashConf.slice(0, dashConf.indexOf('bind=127.0.0.1=onion'));
        expect(beforeOnionBind).to.not.match(/^\s*\[.*\]\s*$/m);
      }
    });

    it(`should leave onion listening off and set no proxy when Tor is disabled on ${network}`, () => {
      config.set('network', network);
      config.set('core.tor.enabled', false);

      const dashConf = renderServiceTemplates(config)['core/dash.conf'];

      expect(dashConf).to.match(/^listenonion=0$/m);
      expect(dashConf.match(/^bind=.*$/gm)).to.deep.equal(['bind=0.0.0.0']);
      expect(dashConf).to.not.match(/^onion=/m);
      expect(dashConf).to.not.match(/^torcontrol=/m);
      expect(dashConf).to.not.match(/^proxy=/m);
    });
  });

  it('should point Core at the sidecar on loopback by default', () => {
    config.set('core.tor.control.password', 'dashmatetest');

    const dashConf = renderServiceTemplates(config)['core/dash.conf'];

    expect(dashConf).to.match(/^onion=127\.0\.0\.1:9050$/m);
    expect(dashConf).to.match(/^listenonion=1$/m);
    expect(dashConf).to.match(/^torcontrol=127\.0\.0\.1:9051$/m);
    expect(dashConf).to.match(/^torpassword=dashmatetest$/m);
    // Clearnet peers, and so quorum traffic, must not go through Tor.
    expect(dashConf).to.not.match(/^proxy=/m);
    expect(dashConf).to.not.match(/^onlynet=/m);
  });

  it('should render a torrc with loopback listeners and a hashed control password', () => {
    config.set('core.tor.control.password', 'dashmatetest');

    const torrc = renderServiceTemplates(config)['core/tor/torrc'];

    expect(torrc).to.match(/^SocksPort 127\.0\.0\.1:9050$/m);
    expect(torrc).to.match(/^ControlPort 127\.0\.0\.1:9051$/m);
    expect(torrc).to.match(/^HashedControlPassword 16:[0-9A-F]{16}60[0-9A-F]{40}$/m);
    expect(torrc).to.not.include('dashmatetest');
  });
});
