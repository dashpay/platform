import { expect } from 'chai';
import HomeDir from '../../../src/config/HomeDir.js';
import getBaseConfigFactory from '../../../configs/defaults/getBaseConfigFactory.js';
import renderTemplateFactory from '../../../src/templates/renderTemplateFactory.js';
import renderServiceTemplatesFactory from '../../../src/templates/renderServiceTemplatesFactory.js';

describe('Core template', () => {
  let config;
  let renderServiceTemplates;

  beforeEach(() => {
    config = getBaseConfigFactory(HomeDir.createTemp())();
    renderServiceTemplates = renderServiceTemplatesFactory(renderTemplateFactory());
  });

  // Drive and DAPI read `service`, `platformP2PPort` and `platformHTTPPort` from
  // Core's masternode list RPCs. Core v24 leaves them out unless the deprecated
  // fields are requested, and Platform cannot process a masternode list without
  // them.
  ['mainnet', 'testnet', 'devnet', 'local'].forEach((network) => {
    it(`should keep Core's deprecated masternode service fields in RPC output on ${network}`, () => {
      config.set('network', network);

      const dashConf = renderServiceTemplates(config)['core/dash.conf'];

      // Options under a [section] header apply to that chain only, so the
      // option has to be in the default section to cover every network.
      const [defaultSection] = dashConf.split(/^\[/m);

      expect(defaultSection).to.match(/^deprecatedrpc=service$/m);
    });
  });
});
