import { asValue } from 'awilix';
import createDIContainer from '../../src/createDIContainer.js';
import HomeDir from '../../src/config/HomeDir.js';
import wait from '../../src/util/wait.js';

/**
 * Brings up a three validator local network with frequent Drive snapshots,
 * then joins a fresh platform-enabled full node with Tenderdash state sync
 * enabled and asserts it bootstraps from a snapshot instead of replaying
 * blocks (earliest_block_height > 1 while catching_up is false).
 *
 * Protocol version note: dashmate puts no `consensus_params.version` into a
 * local network's Tenderdash genesis, so drive-abci's init_chain receives
 * app_version 0 and starts the chain at PlatformVersion::desired(), the
 * latest known protocol version (>= v15). Reduced platform state is thus
 * written from the genesis block on and every snapshot the validators serve
 * is restorable. No initial protocol version plumbing is required.
 */
describe('Local Network State Sync', function main() {
  this.timeout(60 * 60 * 1000); // 60 minutes
  this.bail(true); // bail on first failure

  let homeDir;
  let container;
  let configGroup;
  let configFile;
  let configFileRepository;
  let writeConfigTemplates;
  let assertLocalServicesRunning;
  let dockerCompose;
  let joinConfig;

  const groupName = 'local';
  const joinConfigName = 'local_join';

  // How often validators create snapshot checkpoints
  // (the config schema minimum is 60 seconds)
  const snapshotFrequencySeconds = 60;

  // DB_PATH in docker-compose.yml plus the default checkpoints subdirectory
  const driveCheckpointsPath = '/var/lib/dash/rs-drive-abci/db/checkpoints';

  /**
   * List heights of snapshot checkpoints a node's Drive has created so far
   *
   * @param {Config} config
   * @return {Promise<number[]>}
   */
  async function getCheckpointHeights(config) {
    let commandOutput;
    try {
      commandOutput = await dockerCompose.execCommand(
        config,
        'drive_abci',
        ['sh', '-c', `ls ${driveCheckpointsPath} 2>/dev/null || true`],
      );
    } catch {
      return [];
    }

    return commandOutput.out
      .split('\n')
      .map((line) => parseInt(line.trim(), 10))
      .filter((height) => Number.isInteger(height) && height > 0);
  }

  /**
   * Fetch sync_info from a node's Tenderdash RPC
   *
   * @param {Config} config
   * @return {Promise<Object>}
   */
  async function getTenderdashSyncInfo(config) {
    let host = config.get('platform.drive.tenderdash.rpc.host');

    if (host === '0.0.0.0') {
      host = '127.0.0.1';
    }

    const port = config.get('platform.drive.tenderdash.rpc.port');

    const response = await fetch(`http://${host}:${port}/status`);

    const { result, sync_info: syncInfo } = await response.json();

    // Tenderdash wraps the response into `result` over HTTP JSON RPC
    return result ? result.sync_info : syncInfo;
  }

  before(async () => {
    container = await createDIContainer();

    homeDir = container.resolve('homeDir');
    if (process.env.DASHMATE_E2E_TESTS_LOCAL_HOMEDIR) {
      homeDir.change(new HomeDir(process.env.DASHMATE_E2E_TESTS_LOCAL_HOMEDIR));
    } else {
      homeDir.change(HomeDir.createTemp());
    }

    // Create config file
    /**
     * @type {ConfigFileJsonRepository}
     */
    configFileRepository = container.resolve('configFileRepository');

    const createConfigFile = container.resolve('createConfigFile');

    if (process.env.DASHMATE_E2E_TESTS_LOCAL_HOMEDIR) {
      configFile = configFileRepository.read();
    } else {
      configFile = createConfigFile();
    }

    // Update local config template that will be used to setup nodes
    // (and to create the join node config later)
    const localConfig = configFile.getConfig(groupName);

    if (process.env.DASHMATE_E2E_TESTS_SKIP_IMAGE_BUILD !== 'true') {
      localConfig.set('dashmate.helper.docker.build.enabled', true);
      localConfig.set('platform.drive.abci.docker.build.enabled', true);
      localConfig.set('platform.dapi.rsDapi.docker.build.enabled', true);
    }

    // Offset from localNetwork.spec.js ports so leftovers of one suite
    // don't collide with the other on a developer machine
    localConfig.set('docker.network.subnet', '172.31.0.0/24');
    localConfig.set('dashmate.helper.api.port', 41000);
    localConfig.set('core.p2p.port', 41001);
    localConfig.set('core.rpc.port', 41002);
    localConfig.set('platform.gateway.listeners.dapiAndDrive.port', 41003);
    localConfig.set('platform.drive.tenderdash.p2p.port', 41004);
    localConfig.set('platform.drive.tenderdash.rpc.port', 41005);
    localConfig.set('platform.drive.tenderdash.pprof.port', 41006);

    container.register({
      configFile: asValue(configFile),
    });

    writeConfigTemplates = container.resolve('writeConfigTemplates');
    assertLocalServicesRunning = container.resolve('assertLocalServicesRunning');
    dockerCompose = container.resolve('dockerCompose');
  });

  describe('setup', () => {
    it('should setup local network', async function testSetup() {
      if (process.env.DASHMATE_E2E_TESTS_LOCAL_HOMEDIR) {
        this.skip('local network set up is provided');
      }

      const setupLocalPresetTask = await container.resolve('setupLocalPresetTask');
      const setupTask = setupLocalPresetTask();

      await setupTask.run({
        nodeCount: 3,
        debugLogs: true,
        minerInterval: '2.5m',
        isVerbose: true,
      });

      const configExists = configFile.isGroupExists(groupName);

      expect(configExists).to.be.true();

      // Write configs
      await configFileRepository.write(configFile);

      const writtenConfigGroup = configFile.getGroupConfigs(groupName);
      writtenConfigGroup.forEach(writeConfigTemplates);
    });

    it('should enable frequent snapshots on the validators', async () => {
      configGroup = configFile.getGroupConfigs(groupName)
        .filter((config) => config.getName() !== joinConfigName);

      for (const config of configGroup) {
        if (config.get('platform.enable')) {
          // The local preset disables snapshot serving because a network
          // where every node starts from genesis has nothing to sync from.
          // This test is exactly about a node joining later, so turn it on
          // with the lowest allowed frequency.
          config.set('platform.drive.abci.stateSync.snapshots.enabled', true);
          config.set(
            'platform.drive.abci.stateSync.snapshots.frequencySeconds',
            snapshotFrequencySeconds,
          );

          // Produce empty blocks often enough that checkpoints appear and
          // the joiner catches up without waiting minutes between blocks
          config.set('platform.drive.tenderdash.consensus.createEmptyBlocksInterval', '30s');
        }
      }

      await configFileRepository.write(configFile);

      configGroup.forEach(writeConfigTemplates);
    });

    after(() => {
      container.register({
        configGroup: asValue(configGroup),
      });
    });
  });

  describe('start', () => {
    it('should start local network', async () => {
      const startGroupNodesTask = await container.resolve('startGroupNodesTask');
      const task = startGroupNodesTask(configGroup);

      await task.run({
        isVerbose: true,
        waitForReadiness: true,
      });

      await assertLocalServicesRunning(configGroup);
    });
  });

  describe('join node', () => {
    it('should create a snapshot beyond genesis on a validator', async () => {
      const validatorConfig = configGroup.find((config) => config.get('platform.enable'));

      // Wait until a checkpoint above height 1 exists so the joining node
      // demonstrably restores a snapshot instead of replaying from genesis
      const deadline = Date.now() + (15 * 60 * 1000);

      let checkpointHeights = [];
      while (Date.now() < deadline) {
        checkpointHeights = await getCheckpointHeights(validatorConfig);

        if (checkpointHeights.some((height) => height > 1)) {
          break;
        }

        await wait(5000);
      }

      expect(
        checkpointHeights.some((height) => height > 1),
        `no snapshot checkpoint above height 1 on ${validatorConfig.getName()},`
          + ` found: [${checkpointHeights.join(', ')}]`,
      ).to.be.true();
    });

    it('should setup and start a join node', async () => {
      // A leftover config from a previous run against the same home dir
      if (configFile.isConfigExists(joinConfigName)) {
        configFile.removeConfig(joinConfigName);
      }

      const setupLocalJoinNodeTask = container.resolve('setupLocalJoinNodeTask');

      await setupLocalJoinNodeTask(configGroup).run({
        isVerbose: true,
        joinNodeConfigName: joinConfigName,
      });

      joinConfig = configFile.getConfig(joinConfigName);

      expect(joinConfig.get('platform.drive.tenderdash.stateSync.enabled')).to.be.true();

      await configFileRepository.write(configFile);

      writeConfigTemplates(joinConfig);

      const startNodeTask = container.resolve('startNodeTask');

      await startNodeTask(joinConfig).run({
        isVerbose: true,
      });

      await assertLocalServicesRunning([joinConfig]);
    });

    it('should state sync the join node instead of replaying blocks', async () => {
      const deadline = Date.now() + (20 * 60 * 1000);

      let syncInfo;
      while (Date.now() < deadline) {
        try {
          syncInfo = await getTenderdashSyncInfo(joinConfig);

          if (syncInfo
            && syncInfo.catching_up === false
            && parseInt(syncInfo.latest_block_height, 10) > 0) {
            break;
          }
        } catch {
          // Tenderdash RPC is not reachable yet
        }

        await wait(5000);
      }

      expect(syncInfo, 'join node Tenderdash never responded on RPC').to.exist();
      expect(syncInfo.catching_up, 'join node is still catching up').to.be.false();
      expect(parseInt(syncInfo.latest_block_height, 10)).to.be.above(0);

      // A node bootstrapped from a state sync snapshot has a truncated
      // block history starting at the snapshot height. A node that had
      // block synced (replayed) instead would report 1.
      expect(
        parseInt(syncInfo.earliest_block_height, 10),
        'join node replayed blocks from genesis instead of state syncing',
      ).to.be.above(1);

      // Drive and the other services survived applying the snapshot
      await assertLocalServicesRunning([joinConfig]);

      // Drive serves the restored state: DAPI can fetch a system contract
      const waitForNodeToBeReadyTask = container.resolve('waitForNodeToBeReadyTask');
      await waitForNodeToBeReadyTask(joinConfig).run();
    });
  });

  describe('stop', () => {
    it('should stop join node and local network', async () => {
      const stopNodeTask = await container.resolve('stopNodeTask');

      for (const config of [joinConfig, ...configGroup.slice().reverse()]) {
        const task = stopNodeTask(config);
        await task.run({
          isVerbose: true,
          isForce: true,
        });
      }

      await assertLocalServicesRunning([...configGroup, joinConfig], false);
    });
  });

  describe('reset', () => {
    it('should reset local network', async () => {
      const resetNodeTask = await container.resolve('resetNodeTask');

      // The join node carries the same group name, so it is included
      for (const config of configFile.getGroupConfigs(groupName)) {
        const resetTask = resetNodeTask(config);

        await resetTask.run({
          isVerbose: true,
          isHardReset: false,
          isForce: true,
        });
      }

      homeDir.remove();
    });
  });
});
