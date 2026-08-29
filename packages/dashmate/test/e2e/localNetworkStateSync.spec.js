import { asValue } from 'awilix';
import createDIContainer from '../../src/createDIContainer.js';
import HomeDir from '../../src/config/HomeDir.js';
import wait from '../../src/util/wait.js';
import {
  createFaucetClient,
  createFundedClient,
  mintToNewAddress,
  resetEvoSdkCache,
} from './lib/platformSdk.js';
import seedPlatformState, {
  describeSeedManifest,
  getUnexpectedSkips,
} from './lib/seedPlatformState.js';
import verifySeededState, { describeVerification } from './lib/verifySeededState.js';
import {
  getStateSyncLogExcerpt,
  waitForStateSyncActivity,
  watchStateSync,
} from './lib/stateSyncStatus.js';

/**
 * Brings up a three validator local network with frequent Drive snapshots,
 * seeds real state onto it, then exercises Tenderdash state sync against that
 * chain from several angles:
 *
 *  - a fresh node joins and bootstraps from a snapshot instead of replaying
 *    blocks, and the state it restored is re-read from it with proofs;
 *  - a second joiner survives the serving validator being restarted mid-sync;
 *  - a joined node whose platform data is wiped syncs again from scratch;
 *  - a joiner pointed at a network with snapshot serving turned off falls back
 *    to block sync rather than hanging.
 *
 * The scenarios share one network on purpose. Each bring-up costs many minutes
 * and the later scenarios only need a config change on the running validators,
 * so re-running setup for each would multiply the wall clock for no extra
 * coverage. They do have to run in order: the fallback scenario disables
 * snapshot serving and never turns it back on.
 *
 * No protocol version plumbing is needed for restorable snapshots: local
 * network genesis carries no app_version, so drive-abci starts the chain at
 * PlatformVersion::desired() (latest, >= v15) and writes reduced platform
 * state from the genesis block on.
 */
describe('Local Network State Sync', function main() {
  this.timeout(120 * 60 * 1000); // 120 minutes
  this.bail(true); // bail on first failure

  let homeDir;
  let container;
  let configGroup;
  let configFile;
  let configFileRepository;
  let writeConfigTemplates;
  let assertLocalServicesRunning;
  let dockerCompose;
  let docker;
  let joinConfig;
  let churnConfig;
  let fallbackConfig;
  let seedManifest;

  const groupName = 'local';
  const joinConfigName = 'local_join';
  const churnConfigName = 'local_join_churn';
  const fallbackConfigName = 'local_join_fallback';

  const joinConfigNames = [joinConfigName, churnConfigName, fallbackConfigName];

  // How often validators create snapshot checkpoints
  // (the config schema minimum is 60 seconds)
  const snapshotFrequencySeconds = 60;

  // DB_PATH in docker-compose.yml plus the default checkpoints subdirectory
  const driveCheckpointsPath = '/var/lib/dash/rs-drive-abci/db/checkpoints';

  // Host resources this run may claim. Overridable so two checkouts (or a run
  // following one whose docker networks were left behind) can coexist.
  const subnet = process.env.DASHMATE_E2E_STATE_SYNC_SUBNET || '172.31.0.0/24';
  const portBase = parseInt(process.env.DASHMATE_E2E_STATE_SYNC_PORT_BASE || '41000', 10);
  const auxPortBase = parseInt(process.env.DASHMATE_E2E_STATE_SYNC_AUX_PORT_BASE || '42000', 10);

  /**
   * Everything worth putting in a run report that assertions alone do not
   * carry: seeding outcomes, mid-sync observations, log excerpts.
   *
   * @type {string[]}
   */
  const report = [];

  /**
   * @param {string} line
   * @return {void}
   */
  function record(line) {
    report.push(line);

    // Mocha swallows stdout from hooks in some reporters, but the e2e suite
    // runs with the spec reporter where this is the run's evidence trail.
    // eslint-disable-next-line no-console
    console.log(`[state-sync-qa] ${line}`);
  }

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
   * Wait until a validator has a snapshot checkpoint above genesis.
   *
   * @param {Config} validatorConfig
   * @param {number} [timeoutMs]
   * @return {Promise<number[]>}
   */
  async function waitForCheckpointAboveGenesis(validatorConfig, timeoutMs = 15 * 60 * 1000) {
    const deadline = Date.now() + timeoutMs;

    let checkpointHeights = [];
    while (Date.now() < deadline) {
      checkpointHeights = await getCheckpointHeights(validatorConfig);

      if (checkpointHeights.some((height) => height > 1)) {
        break;
      }

      await wait(5000);
    }

    return checkpointHeights;
  }

  /**
   * Set up, start and return the config of an extra node joining the network.
   *
   * @param {string} configName
   * @param {number} offsetIndex
   * @return {Promise<Config>}
   */
  async function startJoinNode(configName, offsetIndex) {
    const setupLocalJoinNodeTask = container.resolve('setupLocalJoinNodeTask');

    await setupLocalJoinNodeTask(configGroup, { configName, offsetIndex }).run({
      isVerbose: true,
    });

    const config = configFile.getConfig(configName);

    await configFileRepository.write(configFile);

    writeConfigTemplates(config);

    const startNodeTask = container.resolve('startNodeTask');

    await startNodeTask(config).run({
      isVerbose: true,
    });

    return config;
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
    // don't collide with the other on a developer machine.
    //
    // Both the subnet and the two port blocks are overridable, because a
    // developer machine can already carry an unrelated local network (or an
    // orphaned docker network from an earlier run) sitting on these ranges,
    // and docker refuses to create an overlapping pool.
    localConfig.set('docker.network.subnet', subnet);
    localConfig.set('dashmate.helper.api.port', portBase);
    localConfig.set('core.p2p.port', portBase + 1);
    localConfig.set('core.rpc.port', portBase + 2);
    localConfig.set('platform.gateway.listeners.dapiAndDrive.port', portBase + 3);
    localConfig.set('platform.drive.tenderdash.p2p.port', portBase + 4);
    localConfig.set('platform.drive.tenderdash.rpc.port', portBase + 5);
    localConfig.set('platform.drive.tenderdash.pprof.port', portBase + 6);

    // The remaining host-published ports (see the `ports:` sections in
    // docker-compose.yml) are moved off their defaults too, so the suite can
    // run next to another local network that keeps the stock ports
    localConfig.set('core.zmq.port', auxPortBase + 1);
    localConfig.set('platform.drive.abci.tokioConsole.port', auxPortBase + 2);
    localConfig.set('platform.drive.abci.metrics.port', auxPortBase + 3);
    localConfig.set('platform.drive.abci.grovedbVisualizer.port', auxPortBase + 4);
    localConfig.set('platform.drive.tenderdash.metrics.port', auxPortBase + 5);
    localConfig.set('platform.gateway.metrics.port', auxPortBase + 6);
    localConfig.set('platform.gateway.admin.port', auxPortBase + 7);
    localConfig.set('platform.gateway.rateLimiter.metrics.port', auxPortBase + 8);
    localConfig.set('platform.quorumList.api.port', auxPortBase + 9);

    // Leftover join node configs from a previous run against this home dir
    joinConfigNames.forEach((name) => {
      if (configFile.isConfigExists(name)) {
        configFile.removeConfig(name);
      }
    });

    container.register({
      configFile: asValue(configFile),
    });

    writeConfigTemplates = container.resolve('writeConfigTemplates');
    assertLocalServicesRunning = container.resolve('assertLocalServicesRunning');
    dockerCompose = container.resolve('dockerCompose');
    docker = container.resolve('docker');
  });

  // Teardown lives here rather than in a trailing `it()` because the suite
  // bails on first failure, and Mocha never enters a describe block it has
  // not reached yet. A failure — the thing this suite exists to catch — would
  // otherwise strand three validators, up to three join nodes, their volumes
  // and the docker network, which the next run then collides with on the same
  // subnet and ports. Every step is best-effort so one failure cannot stop the
  // rest of the cleanup.
  after(async function teardown() {
    this.timeout(30 * 60 * 1000);

    const joinConfigs = [joinConfig, churnConfig, fallbackConfig].filter(Boolean);
    const allConfigs = [...joinConfigs, ...(configGroup || []).slice().reverse()];

    if (allConfigs.length > 0) {
      const stopNodeTask = container.resolve('stopNodeTask');

      for (const config of allConfigs) {
        try {
          await stopNodeTask(config).run({ isForce: true });
        } catch (error) {
          record(`teardown: could not stop ${config.getName()}: ${error.message}`);
        }
      }

      // Removes the containers and their volumes
      const resetNodeTask = container.resolve('resetNodeTask');

      for (const config of configFile.getGroupConfigs(groupName)) {
        try {
          await resetNodeTask(config).run({
            isHardReset: false,
            isForce: true,
          });
        } catch (error) {
          record(`teardown: could not reset ${config.getName()}: ${error.message}`);
        }
      }
    }

    try {
      homeDir.remove();
    } catch (error) {
      record(`teardown: could not remove home dir: ${error.message}`);
    }

    if (report.length > 0) {
      // eslint-disable-next-line no-console
      console.log(`\n[state-sync-qa] run report\n${report.map((line) => `  ${line}`).join('\n')}\n`);
    }
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
      configGroup = configFile.getGroupConfigs(groupName);

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

  describe('seed state', () => {
    it('should seed identities, names, contracts and documents', async () => {
      const seedConfig = configGroup.find((config) => config.getName() === 'local_seed');
      const validatorConfig = configGroup.find((config) => config.get('platform.enable'));

      // Mine coins the SDK wallet can spend. dashmate's own wallet task is
      // reused so the isolated home dir and per-suite ports are honoured.
      const { privateKey } = await mintToNewAddress(container, seedConfig, 50);

      const faucetClient = createFaucetClient(validatorConfig, seedConfig, privateKey);

      let client;
      try {
        client = await createFundedClient(
          validatorConfig,
          seedConfig,
          faucetClient,
          800000000,
        );

        seedManifest = await seedPlatformState(client, { log: record });
      } finally {
        await faucetClient.disconnect().catch(() => {});
        if (client) {
          await client.disconnect().catch(() => {});
        }
      }

      record(`seeding outcomes:\n${describeSeedManifest(seedManifest)}`);

      // Individual steps are allowed to skip (tokens have no JS SDK path at
      // all), but a run where nothing landed would make every later state
      // assertion vacuous.
      expect(
        seedManifest.identities.length,
        `no identity was seeded; outcomes:\n${describeSeedManifest(seedManifest)}`,
      ).to.be.above(0);

      expect(
        Object.keys(seedManifest.contracts).length,
        `no data contract was seeded; outcomes:\n${describeSeedManifest(seedManifest)}`,
      ).to.be.above(0);

      expect(
        seedManifest.documents.length,
        `no document was seeded; outcomes:\n${describeSeedManifest(seedManifest)}`,
      ).to.be.above(0);

      // A step that failed for a reason this suite does not already know about
      // is the interesting kind: it may be a real regression wearing a skip's
      // clothing, so it is called out separately from the known token gap.
      const unexpected = getUnexpectedSkips(seedManifest);

      if (unexpected.length > 0) {
        record(`UNEXPECTED seeding skips (${unexpected.length}), review these:`);
        unexpected.forEach(({ name, reason }) => record(`  ${name} — ${reason}`));
      } else {
        record('no unexpected seeding skips');
      }
    });
  });

  describe('join node', () => {
    it('should create a snapshot beyond genesis on a validator', async () => {
      const validatorConfig = configGroup.find((config) => config.get('platform.enable'));

      // Wait until a checkpoint above height 1 exists so the joining node
      // demonstrably restores a snapshot instead of replaying from genesis.
      // Seeding already advanced the chain, so this checkpoint carries the
      // seeded state rather than an empty tree.
      const checkpointHeights = await waitForCheckpointAboveGenesis(validatorConfig);

      expect(
        checkpointHeights.some((height) => height > 1),
        `no snapshot checkpoint above height 1 on ${validatorConfig.getName()},`
          + ` found: [${checkpointHeights.join(', ')}]`,
      ).to.be.true();

      record(`validator checkpoints: [${checkpointHeights.join(', ')}]`);
    });

    it('should setup and start a join node', async () => {
      joinConfig = await startJoinNode(joinConfigName, configGroup.length);

      expect(joinConfig.get('platform.drive.tenderdash.stateSync.enabled')).to.be.true();

      await assertLocalServicesRunning([joinConfig]);
    });

    it('should state sync the join node instead of replaying blocks', async () => {
      const {
        syncInfo,
        tenderdashObservations,
        dapiObservations,
        dapiErrors,
      } = await watchStateSync(joinConfig, { log: record });

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

      record(`joined at earliest_block_height=${syncInfo.earliest_block_height},`
        + ` latest_block_height=${syncInfo.latest_block_height}`);

      // Ops acceptance: the state sync counters an operator would watch.
      // A sync that finishes between two polls legitimately leaves none, so
      // this is recorded rather than asserted.
      record(`mid-sync Tenderdash state sync observations: ${tenderdashObservations.length}`);
      tenderdashObservations.forEach((observation) => {
        record(`  ${JSON.stringify(observation)}`);
      });

      record(`mid-sync DAPI getStatus observations: ${dapiObservations.length}`);
      dapiObservations.slice(0, 5).forEach((observation) => {
        record(`  ${JSON.stringify(observation)}`);
      });

      if (dapiErrors.length > 0) {
        record(`DAPI getStatus errors seen while syncing: ${JSON.stringify(dapiErrors)}`);
      }

      // Drive and the other services survived applying the snapshot
      await assertLocalServicesRunning([joinConfig]);

      // Drive serves the restored state: DAPI can fetch a system contract
      const waitForNodeToBeReadyTask = container.resolve('waitForNodeToBeReadyTask');
      await waitForNodeToBeReadyTask(joinConfig).run();
    });

    it('should serve the seeded state from the joined node with proofs', async () => {
      const checks = await verifySeededState(joinConfig, configGroup[0], seedManifest);

      record(`seeded state on the joined node:\n${describeVerification(checks)}`);

      const missing = checks.filter(({ present }) => !present);

      expect(checks.length, 'nothing was verified against the joined node').to.be.above(0);

      expect(
        missing.length,
        `the joined node did not serve seeded state:\n${describeVerification(missing)}`,
      ).to.equal(0);
    });

    it('should report a healthy node through dashmate status', async () => {
      const getPlatformScope = container.resolve('getPlatformScope');

      const scope = await getPlatformScope(joinConfig);

      record(`dashmate platform status: tenderdash=${scope.tenderdash.serviceStatus}`
        + ` drive=${scope.drive.serviceStatus}`
        + ` height=${scope.tenderdash.latestBlockHeight}`
        + ` peers=${scope.tenderdash.peers}`);

      expect(scope.tenderdash.catchingUp).to.be.false();
      expect(scope.tenderdash.serviceStatus).to.equal('up');
      expect(scope.drive.serviceStatus).to.equal('up');
    });

    it('should capture the sync lifecycle from the joiner logs', async () => {
      const { lines, stateSyncLines } = await getStateSyncLogExcerpt(dockerCompose, joinConfig);

      record(`joiner Tenderdash sync lifecycle log excerpt (${lines.length} lines):`);
      lines.forEach((line) => record(`  ${line}`));

      // Peering and consensus handover lines appear on every Tenderdash start,
      // so only the snapshot/chunk ones can back this assertion.
      expect(
        stateSyncLines.length,
        'no snapshot or chunk lines in the joiner logs',
      ).to.be.above(0);
    });
  });

  describe('serving-side churn', () => {
    it('should complete a sync while the serving validator restarts', async () => {
      churnConfig = await startJoinNode(churnConfigName, configGroup.length + 1);

      // Wait until the joiner is demonstrably restoring a snapshot before
      // disturbing anything. Firing the restart at an arbitrary moment could
      // land before the transfer starts or after it finished, and the
      // scenario would pass having tested nothing.
      const activity = await waitForStateSyncActivity(churnConfig);

      if (activity) {
        record(`churn joiner is mid-restore: ${JSON.stringify(activity)}`);
      } else {
        record('INCONCLUSIVE: the churn joiner never reported an in-progress restore,'
          + ' so the restart below did not interrupt a chunk transfer');
      }

      // Which validator serves the snapshot is decided by Tenderdash's own
      // peer discovery, so restarting one picked at random would leave open
      // whether the serving node was ever touched. Restart them all, one at a
      // time: every candidate server is bounced while the other two keep the
      // chain producing blocks.
      let restarted = 0;

      for (const config of configGroup) {
        if (!config.get('platform.enable')) {
          continue;
        }

        const containerIds = await dockerCompose.getContainerIds(config, {
          filterServiceNames: ['drive_abci', 'drive_tenderdash'],
        });

        for (const id of containerIds) {
          await docker.getContainer(id).restart({ t: 5 });
          restarted += 1;
        }

        record(`restarted ${containerIds.length} platform containers on ${config.getName()}`);
      }

      expect(restarted, 'no validator containers were restarted').to.be.above(0);

      const { syncInfo } = await watchStateSync(churnConfig, { log: record });

      expect(syncInfo, 'churn join node Tenderdash never responded on RPC').to.exist();
      expect(
        syncInfo.catching_up,
        'churn join node did not finish syncing after the serving validators restarted',
      ).to.be.false();

      // Recorded, not asserted: a restart harsh enough to exhaust the state
      // sync retries legitimately leaves the joiner block syncing instead,
      // which is still a completed sync but a different path.
      const earliest = parseInt(syncInfo.earliest_block_height, 10);

      record(`churn joiner reached earliest_block_height=${syncInfo.earliest_block_height},`
        + ` latest_block_height=${syncInfo.latest_block_height}`
        + ` (${earliest > 1 ? 'state synced' : 'fell back to block sync'})`);

      await assertLocalServicesRunning([churnConfig]);
    });

    it('should sync again after the joined node loses its platform data', async () => {
      const stopNodeTask = container.resolve('stopNodeTask');

      await stopNodeTask(joinConfig).run({
        isVerbose: true,
        isForce: true,
        platformOnly: true,
      });

      // Wipe platform volumes only: Core keeps its synced chain, so this is a
      // node that has lost Drive and Tenderdash state and must state sync from
      // scratch, not a brand new node.
      const resetNodeTask = container.resolve('resetNodeTask');

      await resetNodeTask(joinConfig).run({
        isVerbose: true,
        isForce: true,
        isPlatformOnlyReset: true,
        isHardReset: false,
      });

      // A pooled connection to the old container would outlive the wipe
      resetEvoSdkCache();

      writeConfigTemplates(joinConfig);

      const startNodeTask = container.resolve('startNodeTask');

      await startNodeTask(joinConfig).run({
        isVerbose: true,
        platformOnly: true,
      });

      const { syncInfo, tenderdashObservations } = await watchStateSync(joinConfig, {
        log: record,
      });

      expect(syncInfo, 're-joined node Tenderdash never responded on RPC').to.exist();
      expect(syncInfo.catching_up, 're-joined node is still catching up').to.be.false();
      expect(
        parseInt(syncInfo.earliest_block_height, 10),
        're-joined node replayed blocks from genesis instead of state syncing',
      ).to.be.above(1);

      record(`re-joined node reached earliest_block_height=${syncInfo.earliest_block_height}`
        + ` after ${tenderdashObservations.length} state sync observations`);
    });
  });

  describe('fallback ladder', () => {
    it('should fall back to block sync when no validator serves snapshots', async () => {
      // Turn snapshot serving off across the network. drive-abci answers
      // ListSnapshots with an empty set when disabled regardless of the
      // checkpoints still on disk, so a joiner finds nothing to offer.
      for (const config of configGroup) {
        if (config.get('platform.enable')) {
          config.set('platform.drive.abci.stateSync.snapshots.enabled', false);
        }
      }

      await configFileRepository.write(configFile);
      configGroup.forEach(writeConfigTemplates);

      const restartNodeTask = container.resolve('restartNodeTask');

      for (const config of configGroup) {
        if (config.get('platform.enable')) {
          await restartNodeTask(config).run({
            isVerbose: true,
            isForce: true,
            platformOnly: true,
          });
        }
      }

      resetEvoSdkCache();

      await assertLocalServicesRunning(configGroup);

      fallbackConfig = await startJoinNode(fallbackConfigName, configGroup.length + 2);

      expect(fallbackConfig.get('platform.drive.tenderdash.stateSync.enabled')).to.be.true();

      // The joiner discovers no snapshots, exhausts its state sync retries and
      // then block syncs. That path replays every block, so unlike the state
      // synced nodes above it keeps the full history from genesis.
      const { syncInfo } = await watchStateSync(fallbackConfig, {
        log: record,
        timeoutMs: 30 * 60 * 1000,
      });

      expect(syncInfo, 'fallback join node Tenderdash never responded on RPC').to.exist();
      expect(
        syncInfo.catching_up,
        'fallback join node never finished syncing without snapshots',
      ).to.be.false();

      expect(
        parseInt(syncInfo.earliest_block_height, 10),
        'fallback join node did not replay from genesis',
      ).to.equal(1);

      record(`fallback joiner block synced to latest_block_height=${syncInfo.latest_block_height}`
        + ` with earliest_block_height=${syncInfo.earliest_block_height}`);

      const { lines } = await getStateSyncLogExcerpt(dockerCompose, fallbackConfig);

      record(`fallback joiner log excerpt (${lines.length} lines):`);
      lines.slice(-40).forEach((line) => record(`  ${line}`));
    });

    it('should still serve the seeded state after block syncing', async () => {
      const waitForNodeToBeReadyTask = container.resolve('waitForNodeToBeReadyTask');
      await waitForNodeToBeReadyTask(fallbackConfig).run();

      const checks = await verifySeededState(fallbackConfig, configGroup[0], seedManifest);

      record(`seeded state on the block synced node:\n${describeVerification(checks)}`);

      const missing = checks.filter(({ present }) => !present);

      expect(
        missing.length,
        `the block synced node did not serve seeded state:\n${describeVerification(missing)}`,
      ).to.equal(0);
    });
  });

  describe('stop', () => {
    it('should stop join nodes and local network', async () => {
      const stopNodeTask = await container.resolve('stopNodeTask');

      const joinConfigs = [joinConfig, churnConfig, fallbackConfig].filter(Boolean);

      for (const config of [...joinConfigs, ...configGroup.slice().reverse()]) {
        const task = stopNodeTask(config);
        await task.run({
          isVerbose: true,
          isForce: true,
        });
      }

      await assertLocalServicesRunning([...configGroup, ...joinConfigs], false);
    });
  });
});
