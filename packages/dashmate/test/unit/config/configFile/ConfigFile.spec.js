import { expect } from 'chai';
import HomeDir from '../../../../src/config/HomeDir.js';
import getBaseConfigFactory from '../../../../configs/defaults/getBaseConfigFactory.js';
import ConfigFile from '../../../../src/config/configFile/ConfigFile.js';

describe('ConfigFile', () => {
  describe('#createConfig', () => {
    it('should give a new config its own Tor control password', () => {
      const base = getBaseConfigFactory(HomeDir.createTemp())();
      const configFile = new ConfigFile([base], '4.2.0', null, null, null);

      const created = configFile.createConfig('node', 'base');

      const password = created.get('core.tor.control.password');
      expect(password).to.match(/^[A-Za-z0-9]{12}$/);
      expect(password).to.not.equal(base.get('core.tor.control.password'));
      expect(configFile.createConfig('other', 'base').get('core.tor.control.password'))
        .to.not.equal(password);
    });
  });
});
