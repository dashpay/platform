import fs from 'fs';
import os from 'os';
import path from 'path';
import zlib from 'zlib';
import getBaseConfigFactory from '../../../configs/defaults/getBaseConfigFactory.js';
import obfuscateConfig from '../../../src/config/obfuscateConfig.js';
import archiveSamples from '../../../src/doctor/archiveSamples.js';
import Samples from '../../../src/doctor/Samples.js';
import unarchiveSamplesFactory from '../../../src/doctor/unarchiveSamplesFactory.js';
import maskOperatorIdentity, { getOperatorIdentity } from '../../../src/util/maskOperatorIdentity.js';

describe('archiveSamples', () => {
  let archiveRoot;
  let previousTmpDir;
  let config;
  let samples;

  /**
   * A tarball stores member contents verbatim, so the decompressed stream holds
   * every byte of every file in the archive, whatever its name.
   *
   * @return {Promise<string>}
   */
  async function archiveAndRead() {
    await archiveSamples(samples, archiveRoot, config);

    const [archiveName] = fs.readdirSync(archiveRoot)
      .filter((name) => name.endsWith('.tar.gz'));

    return zlib.gunzipSync(fs.readFileSync(path.join(archiveRoot, archiveName))).toString('utf8');
  }

  beforeEach(() => {
    archiveRoot = fs.mkdtempSync(path.join(os.tmpdir(), 'dashmate-archive-test-'));

    // The archive is staged under the system temp directory before it is packed
    previousTmpDir = process.env.TMPDIR;
    process.env.TMPDIR = archiveRoot;

    config = getBaseConfigFactory()();
    samples = new Samples();
  });

  afterEach(() => {
    if (previousTmpDir === undefined) {
      delete process.env.TMPDIR;
    } else {
      process.env.TMPDIR = previousTmpDir;
    }

    fs.rmSync(archiveRoot, { recursive: true, force: true });
  });

  // The command tells the operator the archive carries no secrets, and the
  // obfuscated config sample is only one of the places a secret can reach: a
  // service can log it, an RPC response can echo it. Every secret the config
  // holds is known when the archive is written, so none of them is allowed into
  // any file of it.
  describe('secrets from the config', () => {
    const rpcPassword = 'plantedRpcPassword7Qx';
    const operatorKey = '5a'.repeat(32);
    const nodeKey = 'plantedNodeKeyBase64+/==';
    const zeroSslApiKey = 'plantedZeroSslApiKey'.padEnd(32, '0');
    // Text samples that embed JSON, such as a structured log line, carry this
    // one escaped, so matching only the raw value would miss it there
    const passwordWithQuote = 'planted"Password\\7Qx';

    beforeEach(() => {
      config.set('core.rpc.users.drive_consensus.password', rpcPassword);
      config.set('core.rpc.users.dapi.password', passwordWithQuote);
      config.set('core.masternode.operator.privateKey', operatorKey);
      config.set('platform.drive.tenderdash.node.key', nodeKey);
      config.set('platform.gateway.ssl.providerConfigs.zerossl.apiKey', zeroSslApiKey);

      samples.setDashmateConfig(obfuscateConfig(config));
    });

    it('should scrub a config secret that a service wrote to its log', async () => {
      samples.setServiceInfo(
        'drive_abci',
        'stdOut',
        `2026-11-26T10:00:00Z INFO connecting to core with password ${rpcPassword} at core:9998\n`,
      );

      const archived = await archiveAndRead();

      expect(archived).to.not.contain(rpcPassword);
      // The rest of the line is what makes the log worth sending
      expect(archived).to.contain('INFO connecting to core with password ');
      expect(archived).to.contain(' at core:9998');
    });

    it('should scrub config secrets from every kind of sample', async () => {
      samples.setSystemInfo({ hostname: 'node-1', note: `key ${zeroSslApiKey}` });
      samples.setStringifiedDockerError(`docker failed for ${operatorKey}`);
      samples.setServiceInfo('drive_tenderdash', 'status', {
        result: { nested: [{ value: `node key ${nodeKey}` }] },
      });
      samples.setServiceInfo('rs_dapi', 'dockerInspect', {
        Args: [`--core-rpc-pass=${passwordWithQuote}`],
      });
      samples.setServiceInfo('rs_dapi', 'stdErr', `auth failed for ${passwordWithQuote}\n`);
      samples.setServiceInfo(
        'rs_dapi',
        'stdOut',
        `${JSON.stringify({ msg: 'core auth failed', pass: passwordWithQuote })}\n`,
      );
      samples.setServiceInfo('core', 'stdOut', `Command-line arg: masternodeblsprivkey=${operatorKey}\n`);

      const archived = await archiveAndRead();

      for (const secret of [operatorKey, nodeKey, zeroSslApiKey, passwordWithQuote]) {
        expect(archived).to.not.contain(secret);
        expect(archived).to.not.contain(JSON.stringify(secret).slice(1, -1));
      }

      // Each sample was written, with everything around the secret intact
      expect(archived).to.contain('node-1');
      expect(archived).to.contain('docker failed for ');
      expect(archived).to.contain('node key ');
      expect(archived).to.contain('--core-rpc-pass=');
      expect(archived).to.contain('auth failed for ');
      expect(archived).to.contain('{"msg":"core auth failed","pass":"');
      expect(archived).to.contain('masternodeblsprivkey=');
    });

    it('should mask the whole of a secret that contains another', async () => {
      config.set('core.rpc.users.tenderdash.password', 'nestedSecret1');
      config.set('core.rpc.users.quorum_list.password', 'nestedSecret1Tail9');

      samples.setServiceInfo('core', 'stdOut', 'auth nestedSecret1Tail9 rejected\n');

      const archived = await archiveAndRead();

      expect(archived).to.not.contain('nestedSecret1');
      expect(archived).to.not.contain('Tail9');
      expect(archived).to.contain(`auth ${'*'.repeat('nestedSecret1Tail9'.length)} rejected`);
    });

    // Samples have the operator's name masked out as they are collected, so by
    // the time the archive is written a secret containing the name no longer
    // reads as itself, and the rest of it would go out in clear.
    it('should mask a secret that had the operator name masked out of it', async () => {
      const identity = getOperatorIdentity();
      const secret = `${identity.username}-rpc-secret-7Qx`;

      config.set('core.rpc.users.dapi.password', secret);

      samples.setServiceInfo(
        'rs_dapi',
        'stdOut',
        maskOperatorIdentity(`auth ${secret} rejected\n`, identity),
      );

      const archived = await archiveAndRead();

      expect(archived).to.not.contain('-rpc-secret-7Qx');
      expect(archived).to.contain('auth ');
      expect(archived).to.contain(' rejected');
    });

    it('should only search for config values of at least 8 characters', async () => {
      // A value shorter than that turns up in ordinary text by chance, and
      // masking it there would mangle the logs the report exists to carry
      config.set('core.rpc.users.tenderdash.password', 'version');
      config.set('core.rpc.users.quorum_list.password', 'progress');

      samples.setServiceInfo('core', 'stdOut', 'UpdateTip: version=0x20000000 progress=1.000000\n');

      const archived = await archiveAndRead();

      expect(archived).to.contain('UpdateTip: version=0x20000000 ********=1.000000');
    });

    // Doctor reads a report back to analyse it, so a scrubbed sample still has
    // to parse, and keep its shape, on the way in.
    it('should keep scrubbed JSON samples readable by doctor', async () => {
      samples.setServiceInfo('drive_abci', 'dockerInspect', {
        State: { Restarting: true, ExitCode: 1, Error: `failed with ${passwordWithQuote}` },
        Config: { Labels: { quoted: `"${rpcPassword}"` } },
      });

      await archiveSamples(samples, archiveRoot, config);

      const [archiveName] = fs.readdirSync(archiveRoot)
        .filter((name) => name.endsWith('.tar.gz'));

      const restored = await unarchiveSamplesFactory(() => [{ name: 'drive_abci' }])(
        path.join(archiveRoot, archiveName),
      );

      const inspect = restored.getServiceInfo('drive_abci', 'dockerInspect');

      expect(inspect.State.Restarting).to.be.true();
      expect(inspect.State.ExitCode).to.equal(1);
      expect(inspect.State.Error).to.match(/^failed with \*+$/);
      expect(inspect.Config.Labels.quoted).to.match(/^"\*+"$/);
    });

    // A secret can be spelled like a field name or be all digits. Masking it
    // in the serialized text would rename every such key - the config then
    // fails validation on the way back in - or break numbers that contain it.
    it('should leave keys and numbers alone when a secret looks like one', async () => {
      config.set('core.rpc.users.tenderdash.password', 'password');
      config.set('core.rpc.users.dashmate.password', '12345678');
      samples.setDashmateConfig(obfuscateConfig(config));

      samples.setServiceInfo('drive_abci', 'dockerInspect', {
        State: { Pid: 1234567890, Restarting: false },
      });

      await archiveSamples(samples, archiveRoot, config);

      const [archiveName] = fs.readdirSync(archiveRoot)
        .filter((name) => name.endsWith('.tar.gz'));

      const restored = await unarchiveSamplesFactory(() => [{ name: 'drive_abci' }])(
        path.join(archiveRoot, archiveName),
      );

      expect(restored.getDashmateConfig().get('core.rpc.users.tenderdash.password'))
        .to.equal('********');
      expect(restored.getServiceInfo('drive_abci', 'dockerInspect').State.Pid)
        .to.equal(1234567890);
    });

    it('should refuse to write an archive it cannot remove the secrets from', async () => {
      await expect(archiveSamples(samples, archiveRoot)).to.be.rejectedWith(
        'The config the samples were collected from is needed to remove its secrets',
      );

      expect(fs.readdirSync(archiveRoot)).to.be.empty();
    });
  });
});
