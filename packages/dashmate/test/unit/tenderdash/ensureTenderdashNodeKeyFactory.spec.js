import ensureTenderdashNodeKeyFactory from '../../../src/tenderdash/ensureTenderdashNodeKeyFactory.js';
import renderServiceTemplatesFactory from '../../../src/templates/renderServiceTemplatesFactory.js';
import renderTemplateFactory from '../../../src/templates/renderTemplateFactory.js';
import deriveTenderdashNodeId from '../../../src/tenderdash/deriveTenderdashNodeId.js';
import generateTenderdashNodeKey from '../../../src/tenderdash/generateTenderdashNodeKey.js';
import validateTenderdashNodeKey from '../../../src/listr/prompts/validators/validateTenderdashNodeKey.js';
import Config from '../../../src/config/Config.js';
import HomeDir from '../../../src/config/HomeDir.js';
import getBaseConfigFactory from '../../../configs/defaults/getBaseConfigFactory.js';

describe('ensureTenderdashNodeKeyFactory', () => {
  let config;
  let ensureTenderdashNodeKey;

  const NODE_ID_PATH = 'platform.drive.tenderdash.node.id';
  const NODE_KEY_PATH = 'platform.drive.tenderdash.node.key';

  beforeEach(() => {
    const baseConfig = getBaseConfigFactory(HomeDir.createTemp())();

    // A platform-enabled fullnode assembled outside the wizard
    config = new Config('fullnode', baseConfig.getStoredOptions());
    config.set('platform.enable', true);
    config.set('core.masternode.enable', false);

    ensureTenderdashNodeKey = ensureTenderdashNodeKeyFactory();
  });

  it('should generate a valid node key and id when the key is null', () => {
    expect(config.get(NODE_KEY_PATH)).to.equal(null);

    ensureTenderdashNodeKey(config);

    const key = config.get(NODE_KEY_PATH);

    expect(key).to.be.a('string');
    expect(validateTenderdashNodeKey(key)).to.equal(true);
    expect(config.get(NODE_ID_PATH)).to.equal(deriveTenderdashNodeId(key));
  });

  it('should never regenerate an existing node key', () => {
    const existingKey = generateTenderdashNodeKey();
    const existingId = deriveTenderdashNodeId(existingKey);

    config.set(NODE_ID_PATH, existingId);
    config.set(NODE_KEY_PATH, existingKey);
    config.markAsSaved();

    ensureTenderdashNodeKey(config);

    expect(config.get(NODE_KEY_PATH)).to.equal(existingKey);
    expect(config.get(NODE_ID_PATH)).to.equal(existingId);
    expect(config.isChanged()).to.be.false();
  });

  it('should keep the identity once generated', () => {
    ensureTenderdashNodeKey(config);

    const key = config.get(NODE_KEY_PATH);
    const id = config.get(NODE_ID_PATH);

    // Runs once before saving and once before rendering
    ensureTenderdashNodeKey(config);

    expect(config.get(NODE_KEY_PATH)).to.equal(key);
    expect(config.get(NODE_ID_PATH)).to.equal(id);
  });

  it('should derive a missing node id from an existing key', () => {
    const existingKey = generateTenderdashNodeKey();

    config.set(NODE_KEY_PATH, existingKey);

    ensureTenderdashNodeKey(config);

    expect(config.get(NODE_KEY_PATH)).to.equal(existingKey);
    expect(config.get(NODE_ID_PATH)).to.equal(deriveTenderdashNodeId(existingKey));
  });

  it('should not touch a config with platform disabled', () => {
    config.set('platform.enable', false);
    config.markAsSaved();

    ensureTenderdashNodeKey(config);

    expect(config.get(NODE_KEY_PATH)).to.equal(null);
    expect(config.isChanged()).to.be.false();
  });

  it('should not generate a key for the base template config', () => {
    const baseConfig = new Config('base', config.getStoredOptions());

    ensureTenderdashNodeKey(baseConfig);

    expect(baseConfig.get(NODE_KEY_PATH)).to.equal(null);
    expect(baseConfig.isChanged()).to.be.false();
  });

  it('should not invent an identity for a masternode', () => {
    // An evonode's node id is registered on chain, so a generated one would
    // start a node the network does not recognise instead of failing loudly.
    config.set('core.masternode.enable', true);
    config.markAsSaved();

    ensureTenderdashNodeKey(config);

    expect(config.get(NODE_KEY_PATH)).to.equal(null);
    expect(config.get(NODE_ID_PATH)).to.equal(null);
    expect(config.isChanged()).to.be.false();
  });

  it('should still complete the id of a masternode that has a key', () => {
    const existingKey = generateTenderdashNodeKey();

    config.set('core.masternode.enable', true);
    config.set(NODE_KEY_PATH, existingKey);

    ensureTenderdashNodeKey(config);

    expect(config.get(NODE_ID_PATH)).to.equal(deriveTenderdashNodeId(existingKey));
  });

  it('should render node_key.json with a generated key instead of "null"', () => {
    // Regression: a fullnode configured outside the interactive setup wizard
    // reached template rendering with a null node key, and node_key.json was
    // written with the literal string "null" - tenderdash panicked at startup.
    const renderServiceTemplates = renderServiceTemplatesFactory(
      renderTemplateFactory(),
      ensureTenderdashNodeKey,
    );

    const serviceConfigs = renderServiceTemplates(config);

    const nodeKeyFile = JSON.parse(serviceConfigs['platform/drive/tenderdash/node_key.json']);

    expect(nodeKeyFile.priv_key.value).to.equal(config.get(NODE_KEY_PATH));
    expect(nodeKeyFile.priv_key.value).to.not.equal('null');
    expect(nodeKeyFile.id).to.equal(config.get(NODE_ID_PATH));
    expect(validateTenderdashNodeKey(nodeKeyFile.priv_key.value)).to.equal(true);
  });
});
