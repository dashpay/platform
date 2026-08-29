import HomeDir from '../../../src/config/HomeDir.js';
import ConfigFile from '../../../src/config/configFile/ConfigFile.js';
import { PRESET_LOCAL } from '../../../src/constants.js';
import getBaseConfigFactory from '../../../configs/defaults/getBaseConfigFactory.js';
import getLocalConfigFactory from '../../../configs/defaults/getLocalConfigFactory.js';
import generateTenderdashNodeKey from '../../../src/tenderdash/generateTenderdashNodeKey.js';
import deriveTenderdashNodeId from '../../../src/tenderdash/deriveTenderdashNodeId.js';
import wireLocalTenderdashNode from '../../../src/listr/tasks/setup/local/wireLocalTenderdashNode.js';
import setupLocalJoinNodeTaskFactory from '../../../src/listr/tasks/setup/local/setupLocalJoinNodeTaskFactory.js';

describe('setupLocalJoinNodeTaskFactory', () => {
  const CHAIN_ID = 'dashmate_local_42';
  const EXTERNAL_IP = '192.168.65.2';

  let homeDir;
  let configFile;
  let groupConfigs;
  let platformConfigs;
  let templateConfig;
  let resolveDockerHostIp;
  let obtainSelfSignedCertificateTask;
  let setupLocalJoinNodeTask;

  beforeEach(function beforeEach() {
    homeDir = HomeDir.createTemp();

    const getBaseConfig = getBaseConfigFactory(homeDir);
    templateConfig = getLocalConfigFactory(getBaseConfig)();

    configFile = new ConfigFile(
      [templateConfig],
      '4.2.0',
      'abcdef12',
      null,
      'local',
    );

    // Recreate the shape of an already set up local group:
    // three validators and a platform-disabled seed node
    groupConfigs = ['local_1', 'local_2', 'local_3', 'local_seed']
      .map((name) => configFile.createConfig(name, PRESET_LOCAL));

    groupConfigs.forEach((config, i) => {
      config.set('group', 'local');
      config.set('externalIp', EXTERNAL_IP);
      config.set('core.p2p.port', config.get('core.p2p.port') + (i * 100));

      if (config.getName() === 'local_seed') {
        config.set('platform.enable', false);
        config.set('platform.drive.tenderdash.mode', 'seed');
      } else {
        config.set('platform.drive.tenderdash.mode', 'validator');

        const nodeKey = generateTenderdashNodeKey();
        config.set('platform.drive.tenderdash.node.id', deriveTenderdashNodeId(nodeKey));
        config.set('platform.drive.tenderdash.node.key', nodeKey);
        config.set(
          'platform.drive.tenderdash.p2p.port',
          config.get('platform.drive.tenderdash.p2p.port') + (i * 100),
        );
        config.set('platform.drive.tenderdash.genesis.chain_id', CHAIN_ID);
      }

      config.set('core.spork.address', 'spork-address');
      config.set('core.spork.privateKey', 'spork-private-key');
    });

    platformConfigs = groupConfigs.filter((config) => config.get('platform.enable'));

    resolveDockerHostIp = this.sinon.stub().resolves(EXTERNAL_IP);
    obtainSelfSignedCertificateTask = this.sinon.stub().resolves();

    setupLocalJoinNodeTask = setupLocalJoinNodeTaskFactory(
      configFile,
      resolveDockerHostIp,
      obtainSelfSignedCertificateTask,
    );
  });

  afterEach(() => {
    homeDir.remove();
  });

  describe('wireLocalTenderdashNode', () => {
    it('should wire chain id, peers without self and quorum type', () => {
      const config = groupConfigs[0];

      wireLocalTenderdashNode(config, CHAIN_ID, platformConfigs);

      expect(config.get('platform.drive.tenderdash.genesis.chain_id')).to.equal(CHAIN_ID);

      const persistentPeers = config.get('platform.drive.tenderdash.p2p.persistentPeers');

      expect(persistentPeers).to.have.length(2);
      expect(persistentPeers.map((peer) => peer.id)).to.not.include(
        config.get('platform.drive.tenderdash.node.id'),
      );
      persistentPeers.forEach((peer) => {
        expect(peer.host).to.equal(EXTERNAL_IP);
      });

      // The local preset's validator set quorum is llmqType 106
      expect(config.get('platform.drive.tenderdash.genesis.validator_quorum_type'))
        .to.equal(106);
    });
  });

  describe('setupLocalJoinNodeTask', () => {
    let joinConfig;

    beforeEach(async () => {
      await setupLocalJoinNodeTask(groupConfigs).run();

      joinConfig = configFile.getConfig('local_join');
    });

    it('should create a platform-enabled full node config without masternode', () => {
      expect(joinConfig.get('group')).to.equal('local');
      expect(joinConfig.get('platform.enable')).to.be.true();
      expect(joinConfig.get('platform.drive.tenderdash.mode')).to.equal('full');
      expect(joinConfig.get('core.masternode.enable')).to.be.false();
    });

    it('should enable state sync for the joining node only', () => {
      expect(joinConfig.get('platform.drive.tenderdash.stateSync.enabled')).to.be.true();

      // Snapshot serving stays at the preset default for the joiner
      expect(joinConfig.get('platform.drive.abci.stateSync.snapshots.enabled'))
        .to.equal(templateConfig.get('platform.drive.abci.stateSync.snapshots.enabled'));
    });

    it('should wire the node into the existing Tenderdash network', () => {
      expect(joinConfig.get('platform.drive.tenderdash.genesis.chain_id')).to.equal(CHAIN_ID);

      const persistentPeers = joinConfig.get('platform.drive.tenderdash.p2p.persistentPeers');

      expect(persistentPeers).to.have.length(3);
      expect(persistentPeers.map((peer) => peer.id)).to.have.members(
        platformConfigs.map((config) => config.get('platform.drive.tenderdash.node.id')),
      );
      expect(persistentPeers.map((peer) => peer.port)).to.have.members(
        platformConfigs.map((config) => config.get('platform.drive.tenderdash.p2p.port')),
      );
    });

    it('should use a fresh Tenderdash node identity', () => {
      const nodeId = joinConfig.get('platform.drive.tenderdash.node.id');

      expect(nodeId).to.be.a('string').and.not.empty();
      expect(joinConfig.get('platform.drive.tenderdash.node.key')).to.be.a('string').and.not.empty();

      const validatorNodeIds = platformConfigs
        .map((config) => config.get('platform.drive.tenderdash.node.id'));

      expect(validatorNodeIds).to.not.include(nodeId);
    });

    it('should take the next port offset after the seed node', () => {
      // 3 validators (offsets 0-2) + seed (3) means the joiner continues at 4
      const expectedOffset = 400;

      expect(joinConfig.get('core.p2p.port'))
        .to.equal(templateConfig.get('core.p2p.port') + expectedOffset);
      expect(joinConfig.get('platform.drive.tenderdash.p2p.port'))
        .to.equal(templateConfig.get('platform.drive.tenderdash.p2p.port') + expectedOffset);
      expect(joinConfig.get('platform.drive.tenderdash.rpc.port'))
        .to.equal(templateConfig.get('platform.drive.tenderdash.rpc.port') + expectedOffset);
      expect(joinConfig.get('platform.gateway.listeners.dapiAndDrive.port'))
        .to.equal(templateConfig.get('platform.gateway.listeners.dapiAndDrive.port') + expectedOffset);

      const subnet = joinConfig.get('docker.network.subnet').split('.');
      expect(subnet[2]).to.equal('5');
    });

    it('should join the Core network through the seed node with group sporks', () => {
      const seedConfig = groupConfigs.find((config) => config.getName() === 'local_seed');

      expect(joinConfig.get('core.p2p.seeds')).to.deep.equal([{
        host: EXTERNAL_IP,
        port: seedConfig.get('core.p2p.port'),
      }]);

      expect(joinConfig.get('core.spork.address')).to.equal('spork-address');
      expect(joinConfig.get('core.spork.privateKey')).to.equal('spork-private-key');

      expect(joinConfig.get('core.rpc.users.dashmate.password')).to.not.equal(
        templateConfig.get('core.rpc.users.dashmate.password'),
      );
    });

    it('should obtain a self-signed certificate for the joining node', () => {
      expect(obtainSelfSignedCertificateTask).to.have.been.calledOnce();
      expect(obtainSelfSignedCertificateTask.firstCall.args[0]).to.equal(joinConfig);
    });
  });

  describe('additional join nodes', () => {
    it('should honour an explicit config name and port offset', async () => {
      await setupLocalJoinNodeTask(groupConfigs).run();

      await setupLocalJoinNodeTask(groupConfigs, {
        configName: 'local_join_second',
        offsetIndex: groupConfigs.length + 1,
      }).run();

      const first = configFile.getConfig('local_join');
      const second = configFile.getConfig('local_join_second');

      // A second joiner must not land on the first one's host ports
      expect(second.get('platform.gateway.listeners.dapiAndDrive.port'))
        .to.equal(templateConfig.get('platform.gateway.listeners.dapiAndDrive.port') + 500);
      expect(second.get('platform.drive.tenderdash.rpc.port'))
        .to.equal(templateConfig.get('platform.drive.tenderdash.rpc.port') + 500);

      expect(second.get('platform.gateway.listeners.dapiAndDrive.port'))
        .to.not.equal(first.get('platform.gateway.listeners.dapiAndDrive.port'));

      expect(second.get('docker.network.subnet').split('.')[2]).to.equal('6');

      // ...and must be a distinct Tenderdash node
      expect(second.get('platform.drive.tenderdash.node.id'))
        .to.not.equal(first.get('platform.drive.tenderdash.node.id'));

      expect(second.get('platform.drive.tenderdash.stateSync.enabled')).to.be.true();
    });
  });

  describe('invalid groups', () => {
    it('should fail clearly when the group has no seed node', async () => {
      const groupWithoutSeed = groupConfigs
        .filter((config) => config.getName() !== 'local_seed');

      await expect(setupLocalJoinNodeTask(groupWithoutSeed).run())
        .to.be.rejectedWith('no local_seed config');
    });

    it('should fail clearly when the group has no platform-enabled nodes', async () => {
      const seedOnlyGroup = groupConfigs
        .filter((config) => config.getName() === 'local_seed');

      await expect(setupLocalJoinNodeTask(seedOnlyGroup).run())
        .to.be.rejectedWith('no platform-enabled configs');
    });
  });
});
