import ensureTenderdashNodeKeyFactory from '../../../src/tenderdash/ensureTenderdashNodeKeyFactory.js';
import renderServiceTemplatesFactory from '../../../src/templates/renderServiceTemplatesFactory.js';
import deriveTenderdashNodeId from '../../../src/tenderdash/deriveTenderdashNodeId.js';
import generateTenderdashNodeKey from '../../../src/tenderdash/generateTenderdashNodeKey.js';
import validateTenderdashNodeKey from '../../../src/listr/prompts/validators/validateTenderdashNodeKey.js';
import Config from '../../../src/config/Config.js';
import createDIContainer from '../../../src/createDIContainer.js';

describe('ensureTenderdashNodeKeyFactory', () => {
  let container;
  let config;
  let storedConfig;
  let configFileRepository;
  let ensureTenderdashNodeKey;

  const NODE_ID_PATH = 'platform.drive.tenderdash.node.id';
  const NODE_KEY_PATH = 'platform.drive.tenderdash.node.key';

  beforeEach(async function beforeEach() {
    container = await createDIContainer();

    const defaultConfigs = container.resolve('defaultConfigs');

    config = new Config('testnet', defaultConfigs.get('testnet').getStoredOptions());
    config.set('platform.enable', true);

    // The stored copy the repository would read back from disk
    storedConfig = new Config('testnet', config.getStoredOptions());

    const configFile = {
      isConfigExists: this.sinon.stub().returns(true),
      getConfig: this.sinon.stub().returns(storedConfig),
    };

    configFileRepository = {
      update: this.sinon.stub().callsFake((mutate) => mutate(configFile)),
    };

    ensureTenderdashNodeKey = ensureTenderdashNodeKeyFactory(configFileRepository);
  });

  it('should generate and persist a valid node key when the stored key is null', () => {
    expect(config.get(NODE_KEY_PATH)).to.equal(null);

    ensureTenderdashNodeKey(config);

    const key = config.get(NODE_KEY_PATH);

    expect(key).to.be.a('string');
    expect(validateTenderdashNodeKey(key)).to.equal(true);
    expect(config.get(NODE_ID_PATH)).to.equal(deriveTenderdashNodeId(key));

    // Persisted into the stored copy so a restart reuses the same identity
    expect(configFileRepository.update).to.have.been.calledOnce();
    expect(storedConfig.get(NODE_KEY_PATH)).to.equal(key);
    expect(storedConfig.get(NODE_ID_PATH)).to.equal(config.get(NODE_ID_PATH));
  });

  it('should never regenerate an existing node key', () => {
    const existingKey = generateTenderdashNodeKey();
    const existingId = deriveTenderdashNodeId(existingKey);

    config.set(NODE_ID_PATH, existingId);
    config.set(NODE_KEY_PATH, existingKey);

    ensureTenderdashNodeKey(config);

    expect(config.get(NODE_KEY_PATH)).to.equal(existingKey);
    expect(config.get(NODE_ID_PATH)).to.equal(existingId);
    expect(configFileRepository.update).to.have.not.been.called();
  });

  it('should derive and persist a missing node id from an existing key', () => {
    const existingKey = generateTenderdashNodeKey();

    config.set(NODE_KEY_PATH, existingKey);
    storedConfig.set(NODE_KEY_PATH, existingKey);

    ensureTenderdashNodeKey(config);

    expect(config.get(NODE_KEY_PATH)).to.equal(existingKey);
    expect(config.get(NODE_ID_PATH)).to.equal(deriveTenderdashNodeId(existingKey));
    expect(storedConfig.get(NODE_ID_PATH)).to.equal(deriveTenderdashNodeId(existingKey));
  });

  it('should adopt an identity another process stored first', () => {
    const winningKey = generateTenderdashNodeKey();
    const winningId = deriveTenderdashNodeId(winningKey);

    storedConfig.set(NODE_ID_PATH, winningId);
    storedConfig.set(NODE_KEY_PATH, winningKey);

    ensureTenderdashNodeKey(config);

    expect(config.get(NODE_KEY_PATH)).to.equal(winningKey);
    expect(config.get(NODE_ID_PATH)).to.equal(winningId);
  });

  it('should not touch a config with platform disabled', () => {
    config.set('platform.enable', false);

    ensureTenderdashNodeKey(config);

    expect(config.get(NODE_KEY_PATH)).to.equal(null);
    expect(configFileRepository.update).to.have.not.been.called();
  });

  it('should not generate a key for the base template config', () => {
    const baseConfig = new Config('base', config.getStoredOptions());

    ensureTenderdashNodeKey(baseConfig);

    expect(baseConfig.get(NODE_KEY_PATH)).to.equal(null);
    expect(configFileRepository.update).to.have.not.been.called();
  });

  it('should render node_key.json with a generated key instead of "null"', () => {
    // Regression: a fullnode configured outside the interactive setup wizard
    // reached template rendering with a null node key, and node_key.json was
    // written with the literal string "null" - tenderdash panicked at startup.
    const renderTemplate = container.resolve('renderTemplate');
    const renderServiceTemplates = renderServiceTemplatesFactory(
      renderTemplate,
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
