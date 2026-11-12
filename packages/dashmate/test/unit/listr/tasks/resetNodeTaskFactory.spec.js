import { Listr } from 'listr2';
import HomeDir from '../../../../src/config/HomeDir.js';
import DefaultConfigs from '../../../../src/config/DefaultConfigs.js';
import ConfigFile from '../../../../src/config/configFile/ConfigFile.js';
import getBaseConfigFactory from '../../../../configs/defaults/getBaseConfigFactory.js';
import getTestnetConfigFactory from '../../../../configs/defaults/getTestnetConfigFactory.js';
import resetNodeTaskFactory from '../../../../src/listr/tasks/resetNodeTaskFactory.js';
import generateTenderdashNodeKey from '../../../../src/tenderdash/generateTenderdashNodeKey.js';
import deriveTenderdashNodeId from '../../../../src/tenderdash/deriveTenderdashNodeId.js';

describe('resetNodeTaskFactory', () => {
  let homeDir;
  let config;
  let resetNodeTask;

  beforeEach(() => {
    homeDir = HomeDir.createTemp();

    const getBaseConfig = getBaseConfigFactory(homeDir);
    const getTestnetConfig = getTestnetConfigFactory(homeDir, getBaseConfig);

    // A testnet Evo fullnode: it runs platform, but no identity is registered
    // on chain for it, so dashmate is the only source of its node key.
    config = getTestnetConfig();
    config.set('core.masternode.enable', false);
    config.set('platform.enable', true);
    const key = generateTenderdashNodeKey();
    config.set('platform.drive.tenderdash.node.key', key);
    config.set('platform.drive.tenderdash.node.id', deriveTenderdashNodeId(key));

    const dockerCompose = {
      isNodeRunning: async () => false,
      down: async () => {},
      rm: async () => {},
      getVolumeNames: async () => [],
    };

    resetNodeTask = resetNodeTaskFactory(
      dockerCompose,
      {},
      null,
      null,
      new DefaultConfigs([getBaseConfig, getTestnetConfig]),
      new ConfigFile([getBaseConfig(), config], '4.1.0', 'abcdef12', 'testnet', null),
      homeDir,
      () => ({}),
      () => [],
    );
  });

  afterEach(() => {
    homeDir.remove();
  });

  function hardReset(ctx) {
    return new Listr([{ task: () => resetNodeTask(config) }], { renderer: 'silent' })
      .run({
        isHardReset: true,
        isForce: true,
        keepData: true,
        removeConfig: false,
        ...ctx,
      });
  }

  // Resetting platform options clears the node identity while the config stays
  // a platform fullnode. The command saves the config before rendering, so a key
  // first generated at render time would never reach config.json and the node
  // ID would change on every render.
  it('should leave a fullnode with a new identity after a platform reset', async () => {
    const { key: previousKey } = config.get('platform.drive.tenderdash.node');

    await hardReset({ isPlatformOnlyReset: true });

    const { key, id } = config.get('platform.drive.tenderdash.node');
    expect(key).to.be.a('string').and.not.equal(previousKey);
    expect(id).to.equal(deriveTenderdashNodeId(key));
  });

  // An evonode must run the identity its operator registered on chain. A key
  // generated here would give it an unregistered node ID.
  it('should not generate an identity when platform is reset on a masternode', async () => {
    config.set('core.masternode.enable', true);

    await hardReset({ isPlatformOnlyReset: true });

    expect(config.get('core.masternode.enable')).to.be.true();
    expect(config.get('platform.drive.tenderdash.node')).to.deep.equal({ id: null, key: null });
  });

  // A full reset restores the testnet preset, which is a masternode: its
  // identity must come from the operator, never from dashmate.
  it('should not generate an identity when a full reset restores a masternode', async () => {
    await hardReset({ isPlatformOnlyReset: false });

    expect(config.get('core.masternode.enable')).to.be.true();
    expect(config.get('platform.drive.tenderdash.node')).to.deep.equal({ id: null, key: null });
  });
});
