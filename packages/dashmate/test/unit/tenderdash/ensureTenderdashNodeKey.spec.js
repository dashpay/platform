import ensureTenderdashNodeKey from '../../../src/tenderdash/ensureTenderdashNodeKey.js';
import renderServiceTemplatesFactory from '../../../src/templates/renderServiceTemplatesFactory.js';
import renderTemplateFactory from '../../../src/templates/renderTemplateFactory.js';
import deriveTenderdashNodeId from '../../../src/tenderdash/deriveTenderdashNodeId.js';
import generateTenderdashNodeKey from '../../../src/tenderdash/generateTenderdashNodeKey.js';
import validateTenderdashNodeKey from '../../../src/listr/prompts/validators/validateTenderdashNodeKey.js';
import Config from '../../../src/config/Config.js';
import HomeDir from '../../../src/config/HomeDir.js';
import getBaseConfigFactory from '../../../configs/defaults/getBaseConfigFactory.js';

describe('ensureTenderdashNodeKey', () => {
  let config;

  const NODE_ID_PATH = 'platform.drive.tenderdash.node.id';
  const NODE_KEY_PATH = 'platform.drive.tenderdash.node.key';

  beforeEach(() => {
    const baseConfig = getBaseConfigFactory(HomeDir.createTemp())();

    // A platform-enabled fullnode assembled outside the wizard
    config = new Config('fullnode', baseConfig.getStoredOptions());
    config.set('platform.enable', true);
    config.set('core.masternode.enable', false);
  });

  it('should render a valid identity from a null key and preserve it on subsequent calls', () => {
    expect(config.get(NODE_KEY_PATH)).to.equal(null);
    const render = renderServiceTemplatesFactory(renderTemplateFactory());
    const nodeKeyFile = JSON.parse(render(config)['platform/drive/tenderdash/node_key.json']);
    const key = config.get(NODE_KEY_PATH);

    expect(key).to.be.a('string');
    expect(validateTenderdashNodeKey(key)).to.equal(true);
    expect(nodeKeyFile.priv_key.value).to.equal(key).and.not.to.equal('null');
    expect(nodeKeyFile.id).to.equal(deriveTenderdashNodeId(key));
    expect(config.get(NODE_ID_PATH)).to.equal(nodeKeyFile.id);

    config.markAsSaved();
    ensureTenderdashNodeKey(config);

    expect(config.get(NODE_KEY_PATH)).to.equal(key);
    expect(config.get(NODE_ID_PATH)).to.equal(nodeKeyFile.id);
    expect(config.isChanged()).to.be.false();
  });

  [false, true].forEach(masternode => {
    [false, true].forEach(hasId => {
      const action = hasId ? 'preserve an existing id' : 'derive a missing id';
      it(`should ${action} without changing the key for a ${masternode ? 'masternode' : 'fullnode'}`, () => {
        const key = generateTenderdashNodeKey();
        const id = deriveTenderdashNodeId(key);
        config.set('core.masternode.enable', masternode);
        config.set(NODE_KEY_PATH, key);
        config.set(NODE_ID_PATH, hasId ? id : null);
        config.markAsSaved();

        ensureTenderdashNodeKey(config);

        expect(config.get(NODE_KEY_PATH)).to.equal(key);
        expect(config.get(NODE_ID_PATH)).to.equal(id);
        expect(config.isChanged()).to.equal(!hasId);
      });
    });
  });

  it('should leave a masternode with an omitted key unchanged', () => {
    config.set('core.masternode.enable', true);
    config.set('platform.drive.tenderdash.node', { id: null });
    config.markAsSaved();

    ensureTenderdashNodeKey(config);

    expect(config.get('platform.drive.tenderdash.node')).to.deep.equal({ id: null });
    expect(config.isChanged()).to.be.false();
  });

  [
    ['disabled platform', 'fullnode', 'platform.enable', false],
    ['base template', 'base', 'platform.enable', true],
    ['masternode', 'fullnode', 'core.masternode.enable', true],
  ].forEach(([label, name, option, value]) => {
    it(`should leave a ${label} with no identity unchanged`, () => {
      config = new Config(name, config.getStoredOptions());
      config.set(option, value);
      config.markAsSaved();

      ensureTenderdashNodeKey(config);

      expect(config.get(NODE_KEY_PATH)).to.equal(null);
      expect(config.get(NODE_ID_PATH)).to.equal(null);
      expect(config.isChanged()).to.be.false();
    });
  });
});
