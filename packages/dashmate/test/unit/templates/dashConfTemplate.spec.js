import { expect } from 'chai';
import HomeDir from '../../../src/config/HomeDir.js';
import getBaseConfigFactory from '../../../configs/defaults/getBaseConfigFactory.js';
import renderTemplateFactory from '../../../src/templates/renderTemplateFactory.js';
import renderServiceTemplatesFactory from '../../../src/templates/renderServiceTemplatesFactory.js';

describe('Core dash.conf template', () => {
  let config;
  let renderServiceTemplates;

  beforeEach(() => {
    config = getBaseConfigFactory(HomeDir.createTemp())();
    renderServiceTemplates = renderServiceTemplatesFactory(renderTemplateFactory());
  });

  ['mainnet', 'testnet', 'devnet', 'local'].forEach((network) => {
    it(`should ask Core for the deprecated RPC fields Platform and dashmate read on ${network}`, () => {
      config.set('network', network);

      const dashConf = renderServiceTemplates(config)['core/dash.conf'];
      // Options before the first network section apply on every network.
      const globalConf = dashConf.split(/^\[.*\]$/m)[0];

      // `service` keeps masternode addresses and Platform ports in RPC output, and `softforks`
      // keeps deployments in getblockchaininfo, where `dashmate status platform` reads mn_rr.
      expect(globalConf.match(/^deprecatedrpc=.*$/gm)).to.deep.equal([
        'deprecatedrpc=service',
        'deprecatedrpc=softforks',
      ]);
    });
  });
});
