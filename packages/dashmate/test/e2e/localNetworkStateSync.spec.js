import { asValue } from 'awilix';
import createDIContainer from '../../src/createDIContainer.js';
import HomeDir from '../../src/config/HomeDir.js';
import wait from '../../src/util/wait.js';
import {
  activateLocalSporks,
  createClient,
  fundClientFromCore,
  getCoreHeight,
  mintToNewAddress,
  resetEvoSdkCache,
} from './lib/platformSdk.js';
import seedPlatformState, {
  describeSeedManifest,
  getUnexpectedSkips,
} from './lib/seedPlatformState.js';
import verifySeededState, { describeVerification } from './lib/verifySeededState.js';
import {
  getContainerStates,
  getServiceLogTail,
  getStateSyncLogExcerpt,
  getStateSyncRestoreHeight,
  getTenderdashStatus,
  getTenderdashSyncInfo,
  waitForDapiReady,
  waitForStateSyncActivity,
  watchStateSync,
} from './lib/stateSyncStatus.js';

/**
 * Brings up a three validator local network with frequent Drive snapshots,
 * seeds real state onto it, then exercises Tenderdash state sync against that
 * chain from several angles:
 *
 *  - a fresh node joins and bootstraps from a snapshot instead of replaying
 *    blocks, ends up with the truncated block history state sync promises
 *    (earliest_block_height well above genesis), and the state it restored is
 *    re-read from it with proofs;
 *  - a second joiner survives the serving validator being restarted mid-sync;
 *  - a joined node whose platform data is wiped syncs again from scratch;
 *  - a joiner pointed at a network with snapshot serving turned off falls back
 *    to block sync rather than hanging — and keeps the full history from
 *    genesis, the contrast that makes the truncation assertion meaningful.
 *
 * The truncated history is only observable because setup shrinks the evidence
 * window in the network's genesis (see the evidence constants below). With the
 * stock 100000-block / 48-hour window Tenderdash backfills light blocks from
 * the snapshot all the way to genesis on any local-sized chain, and
 * earliest_block_height lands at 1 even after a genuine restore. Shrinking the
 * window in the shared setup instead of adding a separate truncation spec
 * keeps the property asserted on every scenario here without paying for a
 * second multi-hour network bring-up.
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

  // Starts empty so the post-sync state checks still run (against the genesis
  // state) on a run where seeding could not proceed.
  let seedManifest = { steps: [], identities: [], contracts: {}, documents: [] };
  let seedBlocker;

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

  // Blocks the chain must have produced beyond the newest snapshot before a
  // joiner is asked to restore it
  const SNAPSHOT_HEIGHT_HEADROOM = 10;

  // Evidence window rendered into the network's genesis. After restoring a
  // snapshot Tenderdash backfills light blocks below it until BOTH limits are
  // covered, and only then does earliest_block_height settle. The window is
  // shrunk so the backfill floor (snapshot height minus the window) sits well
  // above genesis and the truncated history can be asserted. The duration leg
  // is kept far below one empty-block interval (30s, set below) so the block
  // count is the binding limit and the expected floor stays computable.
  const EVIDENCE_MAX_AGE_NUM_BLOCKS = 10;
  const EVIDENCE_MAX_AGE_DURATION_NS = '10000000000'; // 10 seconds

  // How far past the block-count window the backfill may legitimately run:
  // the duration leg keeps it going while the blocks it walks are younger
  // than EVIDENCE_MAX_AGE_DURATION_NS, which can add a few blocks when the
  // ones below the snapshot were minted in a seeding burst rather than on the
  // empty-block cadence.
  const BACKFILL_SLACK_BLOCKS = 8;

  // The first joiner must restore a snapshot at least this high so the
  // backfill floor is unambiguously above genesis even with the slack spent
  // (20 - 10 - 8 = 2 > 1). Kept as low as that bound allows: on a run where
  // seeding could not advance the chain, every extra block here is 30 wall
  // clock seconds of empty-block waiting.
  const TRUNCATION_MIN_SNAPSHOT_HEIGHT = 20;

  /**
   * When each node last started, so a restore can be attributed to this boot
   * rather than to an earlier one whose log lines the container still holds.
   *
   * @type {Map<string, string>}
   */
  const bootedAt = new Map();

  /**
   * Note that a node is starting now, a little in the past to absorb any skew
   * between this process's clock and the docker daemon's.
   *
   * @param {Config} config
   * @return {void}
   */
  function markBoot(config) {
    bootedAt.set(config.getName(), new Date(Date.now() - 5000).toISOString());
  }

  /**
   * The snapshot height a node restored during its current boot, or undefined
   * if it restored nothing.
   *
   * @param {Config} config
   * @return {Promise<number|undefined>}
   */
  function getRestoreHeightSinceBoot(config) {
    return getStateSyncRestoreHeight(dockerCompose, config, {
      since: bootedAt.get(config.getName()),
    });
  }

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
   * Wait until a validator has a snapshot checkpoint at or above the given
   * height.
   *
   * @param {Config} validatorConfig
   * @param {number} minHeight
   * @param {number} [timeoutMs]
   * @return {Promise<number[]>}
   */
  async function waitForRestorableCheckpoint(
    validatorConfig,
    minHeight,
    timeoutMs = 25 * 60 * 1000,
  ) {
    const deadline = Date.now() + timeoutMs;

    let checkpointHeights = [];
    while (Date.now() < deadline) {
      checkpointHeights = await getCheckpointHeights(validatorConfig);

      const restorable = checkpointHeights.filter((height) => height >= minHeight);

      if (restorable.length > 0) {
        // A snapshot needs headroom below the tip: Tenderdash verifies the
        // light block at the snapshot height before accepting an offer, and on
        // a chain only a block or two long there is nothing to verify against,
        // so the joiner gives up on discovery and block syncs instead.
        //
        // The headroom is required above the OLDEST acceptable checkpoint,
        // not the newest: snapshots appear about every two blocks here (60s
        // frequency against a 30s empty-block cadence), so the tip can never
        // outrun the newest checkpoint by much — a gate on the newest would
        // wait forever on a quiet chain. The joiner prefers the newest offer
        // and Tenderdash falls back to older snapshots when one cannot be
        // verified, so one acceptable checkpoint with headroom is what the
        // scenario actually needs; the chain also keeps growing during the
        // joiner's multi-minute setup, giving the newer offers their own
        // headroom by the time discovery happens.
        let latestHeight = 0;

        try {
          const syncInfo = await getTenderdashSyncInfo(validatorConfig);
          latestHeight = parseInt(syncInfo.latest_block_height, 10);
        } catch {
          // validator RPC not reachable yet
        }

        if (latestHeight >= Math.min(...restorable) + SNAPSHOT_HEIGHT_HEADROOM) {
          break;
        }
      }

      await wait(5000);
    }

    return checkpointHeights;
  }

  /**
   * Record everything needed to explain a joiner that failed to sync.
   *
   * A sync assertion that fails with nothing but "never answered RPC" cannot
   * be acted on, and by the time the suite tears down the containers are gone.
   *
   * @param {Config} config
   * @param {string} reason
   * @return {Promise<void>}
   */
  async function dumpJoinerDiagnostics(config, reason) {
    record(`DIAGNOSTICS for ${config.getName()}: ${reason}`);

    const states = await getContainerStates(dockerCompose, config);
    record('  container states:');
    states.forEach((line) => record(`    ${line}`));

    for (const service of ['drive_tenderdash', 'drive_abci', 'core']) {
      const lines = await getServiceLogTail(dockerCompose, config, service);
      record(`  last ${lines.length} ${service} log lines:`);
      lines.forEach((line) => record(`    ${line}`));
    }
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

    markBoot(config);

    await startNodeTask(config).run({
      isVerbose: true,
    });

    // A node that joins after setup never learns the local network's sporks,
    // and without SPORK_19 its Core reports no ChainLock, drive-abci waits for
    // one forever and Tenderdash never completes the ABCI handshake. Activate
    // them on the joiner with the group's spork key, exactly as setup does for
    // the original nodes.
    const sporks = await activateLocalSporks(container, config);

    record(`activated ${sporks.length} sporks on ${config.getName()}`);

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

    // Debugging aid: a failure in this suite is usually only diagnosable from
    // the containers teardown is about to destroy. Keeping them alive is opt
    // in and leaves the network, volumes and home dir for a post-mortem — the
    // operator owns the cleanup.
    if (process.env.DASHMATE_E2E_STATE_SYNC_KEEP_NETWORK === 'true') {
      record('teardown skipped (DASHMATE_E2E_STATE_SYNC_KEEP_NETWORK=true); '
        + `the network is still running under ${homeDir.getPath()}`);

      // eslint-disable-next-line no-console
      console.log(`\n[state-sync-qa] run report\n${report.map((line) => `  ${line}`).join('\n')}\n`);

      return;
    }

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

    it('should enable frequent snapshots and a short evidence window', async () => {
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

          // Shrink the evidence window so post-restore backfill stops above
          // genesis and joiners exhibit the truncated block history this
          // suite asserts on. This must land in genesis before the first
          // start: the genesis.json template renders
          // platform.drive.tenderdash.genesis verbatim, and a genesis is
          // immutable once the chain has started. Joiners inherit it through
          // setupLocalJoinNodeTask, which copies the group genesis.
          config.set('platform.drive.tenderdash.genesis.consensus_params.evidence', {
            max_age: String(EVIDENCE_MAX_AGE_NUM_BLOCKS),
            max_age_num_blocks: String(EVIDENCE_MAX_AGE_NUM_BLOCKS),
            max_age_duration: EVIDENCE_MAX_AGE_DURATION_NS,
          });
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
    it('should seed identities, names, contracts and documents', async function seedState() {
      const seedConfig = configGroup.find((config) => config.getName() === 'local_seed');
      const validatorConfig = configGroup.find((config) => config.get('platform.enable'));

      // Mine coins into the seed node's Core wallet. dashmate's own wallet
      // task is reused so the isolated home dir and per-suite ports are
      // honoured, and it also matures the coinbase outputs before returning.
      const { coreService } = await mintToNewAddress(container, seedConfig, 50);

      // Start the wallet's transaction scan at the tip: everything below it
      // predates the key and only costs time to walk.
      const coreHeight = await getCoreHeight(coreService);

      record(`core height before funding: ${coreHeight}`);

      const client = createClient(validatorConfig, seedConfig);

      try {
        const { address, balance } = await fundClientFromCore(coreService, client, 800000000, {
          timeoutMs: 240000,
          log: record,
        });

        record(`funded seeding wallet ${address} with ${balance} duffs`);

        seedManifest = await seedPlatformState(client, { log: record });
      } catch (error) {
        // Funding the SDK wallet goes through wallet-lib, which learns about
        // its coins from DAPI's Core transaction stream. When that stream does
        // not deliver, nothing can be seeded — but the state sync scenarios
        // that follow are what this suite exists for, and they do not depend
        // on custom state, so the run continues against the genesis state
        // instead of losing every later scenario to a seeding problem.
        seedBlocker = error.message;

        record(`SEEDING BLOCKED — continuing against genesis state only: ${error.message}`);
      } finally {
        await client.disconnect().catch(() => {});
      }

      if (seedBlocker) {
        this.skip();
      }

      record(`seeding outcomes:\n${describeSeedManifest(seedManifest)}`);

      // A seeding step that failed is only diagnosable from the serving
      // side, and teardown destroys the containers before anyone can look.
      if (seedManifest.identities.length === 0) {
        for (const service of ['drive_abci', 'gateway']) {
          const lines = await getServiceLogTail(dockerCompose, validatorConfig, service, 60);
          record(`${validatorConfig.getName()} ${service} log tail (seeding produced no identity):`);
          lines.forEach((line) => record(`  ${line}`));
        }
      }

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
    it('should create a snapshot beyond the evidence window on a validator', async () => {
      const validatorConfig = configGroup.find((config) => config.get('platform.enable'));

      // Wait until a checkpoint comfortably above the evidence window exists,
      // so the joining node demonstrably restores a snapshot instead of
      // replaying from genesis AND its post-restore backfill floor sits above
      // genesis. Seeding already advanced the chain, so this checkpoint
      // carries the seeded state rather than an empty tree.
      const checkpointHeights = await waitForRestorableCheckpoint(
        validatorConfig,
        TRUNCATION_MIN_SNAPSHOT_HEIGHT,
      );

      expect(
        checkpointHeights.some((height) => height >= TRUNCATION_MIN_SNAPSHOT_HEIGHT),
        `no snapshot checkpoint at or above height ${TRUNCATION_MIN_SNAPSHOT_HEIGHT}`
          + ` on ${validatorConfig.getName()},`
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

      if (!syncInfo) {
        await dumpJoinerDiagnostics(joinConfig, 'joiner never answered Tenderdash RPC');
      }

      expect(syncInfo, 'join node Tenderdash never responded on RPC').to.exist();
      expect(syncInfo.catching_up, 'join node is still catching up').to.be.false();
      expect(parseInt(syncInfo.latest_block_height, 10)).to.be.above(0);

      // Proof that the node restored a snapshot rather than executing every
      // block, taken from the ABCI app itself. Block execution cannot produce
      // this log line, so it pins down the path independently of the
      // block-store shape asserted on below.
      const restoreHeight = await getRestoreHeightSinceBoot(joinConfig);

      if (restoreHeight === undefined) {
        // The joiner came up but never restored a snapshot. Whether the
        // validators offered one at all is the whole question, and it is only
        // answerable from both sides' logs.
        await dumpJoinerDiagnostics(joinConfig, 'joiner block synced instead of state syncing');

        const validatorConfig = configGroup.find((config) => config.get('platform.enable'));
        const offered = await getStateSyncLogExcerpt(dockerCompose, validatorConfig);

        record(`serving validator ${validatorConfig.getName()} snapshot log lines:`);
        offered.stateSyncLines.slice(-30).forEach((line) => record(`  ${line}`));

        const heights = await getCheckpointHeights(validatorConfig);
        record(`serving validator checkpoints at failure: [${heights.join(', ')}]`);
      }

      expect(
        restoreHeight,
        'join node replayed blocks from genesis instead of restoring a snapshot',
      ).to.be.a('number');

      expect(restoreHeight, 'snapshot was restored at genesis').to.be.above(1);

      // The user-visible property of state sync: the block history starts at
      // the backfill floor, not at genesis. The status is re-fetched here
      // rather than reusing the poll's last sample, both so the assertion
      // reads the settled post-backfill value and so the raw document lands
      // in the run report (which sync_info/statesync fields this Tenderdash
      // actually populates is itself worthwhile evidence).
      const status = await getTenderdashStatus(joinConfig);

      record(`joiner raw /status after sync: ${JSON.stringify(status)}`);

      const earliestBlockHeight = parseInt(status.sync_info.earliest_block_height, 10);

      expect(
        earliestBlockHeight,
        'a state synced node must have a truncated block history starting above genesis',
      ).to.be.above(1);

      // The bounds below assume the joiner restored a snapshot at or above
      // the minimum the checkpoint gate waited for. Tenderdash may fall back
      // to an older offered snapshot, and one low enough would clip the
      // backfill at genesis — fail that precondition by name rather than as
      // a baffling bounds mismatch.
      expect(
        restoreHeight,
        `the joiner restored a snapshot at ${restoreHeight}, below the`
          + ` ${TRUNCATION_MIN_SNAPSHOT_HEIGHT} the truncation bounds assume`,
      ).to.be.at.least(TRUNCATION_MIN_SNAPSHOT_HEIGHT);

      // ... and the floor is where the evidence window puts it: backfill runs
      // from the snapshot down until the window is covered, so the earliest
      // block sits at snapshot height minus the block-count window, give or
      // take the duration leg (see BACKFILL_SLACK_BLOCKS).
      const expectedEarliest = restoreHeight - EVIDENCE_MAX_AGE_NUM_BLOCKS;

      expect(
        earliestBlockHeight,
        `earliest_block_height=${earliestBlockHeight} is inconsistent with the`
          + ` evidence window: the snapshot restored at ${restoreHeight} puts the`
          + ` backfill floor at ${expectedEarliest} (slack ${BACKFILL_SLACK_BLOCKS} below, 2 above)`,
      ).to.be.within(expectedEarliest - BACKFILL_SLACK_BLOCKS, expectedEarliest + 2);

      record(`joiner restored a snapshot at height ${restoreHeight} and kept a`
        + ` truncated history: earliest_block_height=${earliestBlockHeight}`
        + ` (backfill floor ${expectedEarliest} = ${restoreHeight} - ${EVIDENCE_MAX_AGE_NUM_BLOCKS}),`
        + ` latest_block_height=${status.sync_info.latest_block_height}`);

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
      const ready = await waitForDapiReady(joinConfig);

      record(`joined node DAPI is serving: ${JSON.stringify(ready.chain)}`);
      record(`joined node DAPI StateSync fields: ${JSON.stringify(ready.stateSync)}`);
    });

    it('should serve the seeded state from the joined node with proofs', async () => {
      const checks = await verifySeededState(joinConfig, configGroup[0], seedManifest);

      if (seedBlocker) {
        record('NOTE: seeding was blocked, so this check covers the genesis state only');
      }

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
      const churnRestoreHeight = await getRestoreHeightSinceBoot(churnConfig);

      record(`churn joiner finished at latest_block_height=${syncInfo.latest_block_height}`
        + ` (${churnRestoreHeight === undefined
          ? 'fell back to block sync'
          : `restored a snapshot at height ${churnRestoreHeight}`})`);

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

      // From here on, only a restore logged after this moment counts. The wipe
      // does not necessarily replace the container, so without this the first
      // join's restore line would still be visible and would satisfy the
      // assertion below even if this node never re-synced at all.
      markBoot(joinConfig);

      await startNodeTask(joinConfig).run({
        isVerbose: true,
        platformOnly: true,
      });

      const { syncInfo, tenderdashObservations } = await watchStateSync(joinConfig, {
        log: record,
      });

      expect(syncInfo, 're-joined node Tenderdash never responded on RPC').to.exist();
      expect(syncInfo.catching_up, 're-joined node is still catching up').to.be.false();

      const restoreHeight = await getRestoreHeightSinceBoot(joinConfig);

      expect(
        restoreHeight,
        're-joined node replayed blocks from genesis instead of restoring a snapshot',
      ).to.be.a('number');

      // Freshly fetched so backfill has settled, as in the first join
      const { sync_info: rejoinSyncInfo } = await getTenderdashStatus(joinConfig);

      expect(
        parseInt(rejoinSyncInfo.earliest_block_height, 10),
        'the re-joined node must again have a truncated history starting above genesis',
      ).to.be.above(1);

      record(`re-joined node restored a snapshot at height ${restoreHeight}`
        + ` (earliest_block_height=${rejoinSyncInfo.earliest_block_height})`
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

      // The inverse of the join assertion: with nothing offered there must be
      // no restore at all. Checked from the ABCI app rather than from
      // earliest_block_height, which a backfill would make ambiguous.
      const restoreHeight = await getRestoreHeightSinceBoot(fallbackConfig);

      expect(
        restoreHeight,
        'fallback join node restored a snapshot even though serving was disabled',
      ).to.be.undefined();

      // The contrast that makes the join scenario's truncation assertion
      // meaningful: a block synced node replayed and kept every block
      // (drive-abci never requests pruning through retain_height), so even
      // with the shrunk evidence window its history reaches genesis.
      expect(
        parseInt(syncInfo.earliest_block_height, 10),
        'a block synced node must keep the full history from genesis',
      ).to.equal(1);

      record(`fallback joiner block synced to latest_block_height=${syncInfo.latest_block_height}`
        + ` with earliest_block_height=${syncInfo.earliest_block_height}`
        + ' and no snapshot restore');

      const { lines } = await getStateSyncLogExcerpt(dockerCompose, fallbackConfig);

      record(`fallback joiner log excerpt (${lines.length} lines):`);
      lines.slice(-40).forEach((line) => record(`  ${line}`));
    });

    it('should still serve the seeded state after block syncing', async () => {
      await waitForDapiReady(fallbackConfig);

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
