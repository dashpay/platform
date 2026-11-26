import crypto from 'crypto';
import fs from 'fs';
import os from 'os';
import path from 'path';
import zlib from 'zlib';
import { list } from 'tar';
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

    it('should scrub credentials from config URLs and build arguments out of logs', async () => {
      config.set(
        'platform.gateway.ssl.providerConfigs.letsencrypt.acmeDirectoryUrl',
        'https://acmeUser:acmePassword7Qx@acme.example.com/directory',
      );
      config.set('platform.drive.abci.docker.build.buildArgs', {
        HTTPS_PROXY: 'http://proxyUser:proxyPassword7Qx@proxy.example.com:3128',
      });
      samples.setDashmateConfig(obfuscateConfig(config));

      samples.setServiceInfo(
        'gateway',
        'stdOut',
        'lego: fetching https://acmeUser:acmePassword7Qx@acme.example.com/directory\n'
          + 'lego: retrying with acmePassword7Qx\n',
      );
      samples.setServiceInfo(
        'drive_abci',
        'stdErr',
        'build: using proxy http://proxyUser:proxyPassword7Qx@proxy.example.com:3128\n'
          + 'build: proxy auth proxyPassword7Qx rejected\n',
      );

      const archivePath = await archiveSamples(samples, archiveRoot, config);
      const archived = zlib.gunzipSync(fs.readFileSync(archivePath)).toString('utf8');

      expect(archived).to.not.contain('acmePassword7Qx');
      expect(archived).to.not.contain('proxyPassword7Qx');
      expect(archived).to.contain('@acme.example.com/directory');
      expect(archived).to.contain(`lego: retrying with ${'*'.repeat('acmePassword7Qx'.length)}`);
      expect(archived).to.contain('build: using proxy ');
      expect(archived).to.contain(`build: proxy auth ${'*'.repeat('proxyPassword7Qx'.length)} rejected`);

      // The config copy, with the URL and the build argument hidden, still reads back
      const restored = await unarchiveSamplesFactory(() => [])(archivePath);

      expect(restored.getDashmateConfig().get('platform.gateway.ssl.providerConfigs.letsencrypt.acmeDirectoryUrl'))
        .to.equal('https://********:***************@acme.example.com/directory');
    });

    // Masking one secret must not hide part of another from the search: each
    // is looked for in the text as it was, and every match is masked
    it('should mask the whole of two secrets that overlap in the text', async () => {
      config.set('core.rpc.users.tenderdash.password', 'alpha123456');
      config.set('core.rpc.users.quorum_list.password', '123456beta');

      samples.setServiceInfo('core', 'stdOut', 'auth alpha123456beta rejected\n');

      const archived = await archiveAndRead();

      expect(archived).to.not.contain('beta');
      expect(archived).to.contain(`auth ${'*'.repeat('alpha123456beta'.length)} rejected`);
    });

    it('should scrub a nested password from a log that echoes the URL holding it', async () => {
      const inner = 'https://innerUser:long!Password7Qx@inner.example.com/';
      const outer = `https://acme.example.com/directory?${new URLSearchParams({ upstream: inner })}`;

      config.set('platform.gateway.ssl.providerConfigs.letsencrypt.acmeDirectoryUrl', outer);

      samples.setServiceInfo('gateway', 'stdOut', `lego: using directory ${outer}\n`);

      const archived = await archiveAndRead();

      expect(archived).to.not.contain('long%21Password7Qx');
      expect(archived).to.contain('lego: using directory https://acme.example.com/directory?upstream=');
    });

    // A secret of repeated characters matches at every position of a long
    // run of them; the matches are merged as they are found rather than all
    // kept first
    it('should mask a dense run of matches without keeping each one', async function it() {
      config.set('core.rpc.users.tenderdash.password', '00000000');

      samples.setServiceInfo('core', 'stdOut', `${'0'.repeat(2_000_000)}\nend\n`);

      const push = this.sinon.spy(Array.prototype, 'push');

      const archived = await archiveAndRead();

      push.restore();

      expect(archived).to.contain(`${'*'.repeat(2_000_000)}\nend`);
      expect(push.callCount).to.be.below(1000);
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

    // Certificate dates are judged against the moment the samples were taken,
    // so the date has to survive a secret that happens to read like it.
    it('should keep the collection date when a secret reads like it', async () => {
      config.set('core.rpc.users.tenderdash.password', samples.date.toISOString().slice(0, 10));

      const archivePath = await archiveSamples(samples, archiveRoot, config);

      const restored = await unarchiveSamplesFactory(() => [])(archivePath);

      expect(restored.date.toISOString()).to.equal(samples.date.toISOString());
    });

    it('should refuse to write an archive it cannot remove the secrets from', async () => {
      await expect(archiveSamples(samples, archiveRoot)).to.be.rejectedWith(
        'The config the samples were collected from is needed to remove its secrets',
      );

      expect(fs.readdirSync(archiveRoot)).to.be.empty();
    });
  });

  // The samples are written out as files before they are packed. The archive
  // is the one copy meant to leave the machine, so the staged files are not
  // left behind for whoever else can read the temp directory, and nothing
  // already there is packed into the archive with them.
  describe('staging', () => {
    it('should leave only the archive behind', async () => {
      samples.setServiceInfo('core', 'stdOut', 'log line\n');

      const archivePath = await archiveSamples(samples, archiveRoot, config);

      expect(fs.readdirSync(archiveRoot)).to.deep.equal([path.basename(archivePath)]);
    });

    it('should not pack files it did not write', async () => {
      // Under each name the staging directory has had: the collection time
      // as it is, and with the ':' a Windows file name cannot hold replaced
      const isoDate = samples.date.toISOString();

      for (const timestamp of [isoDate, isoDate.replace(/:/g, '-')]) {
        const presetDir = path.join(archiveRoot, `dashmate-report-${timestamp}`);

        fs.mkdirSync(presetDir);
        fs.writeFileSync(path.join(presetDir, 'planted.txt'), 'content from elsewhere');
      }

      samples.setServiceInfo('core', 'stdOut', 'log line\n');

      const archived = await archiveAndRead();

      expect(archived).to.contain('log line');
      expect(archived).to.not.contain('planted.txt');
      expect(archived).to.not.contain('content from elsewhere');
    });

    it('should not leave the staged files behind when packing fails', async () => {
      samples.setServiceInfo('core', 'stdOut', 'log line\n');

      // Fails where the archive itself is opened, after every sample is staged
      await expect(
        archiveSamples(samples, path.join(archiveRoot, 'missing', 'folder'), config),
      ).to.be.rejectedWith(/ENOENT.*missing/);

      expect(fs.readdirSync(archiveRoot)).to.be.empty();
    });

    // The staging directory is private, and an entry for it would hand that
    // mode to the directory the archive is extracted into
    it('should not pack an entry for the staging directory itself', async () => {
      samples.setServiceInfo('core', 'stdOut', 'log line\n');

      const archivePath = await archiveSamples(samples, archiveRoot, config);

      const entries = [];

      await list({
        file: archivePath,
        onReadEntry: (entry) => entries.push(entry.path),
      });

      expect(entries).to.include('core/stdOut.txt');
      expect(entries.map((entry) => path.posix.normalize(entry))).to.not.include('.');
    });
  });

  // The operator's name is masked out of every sample, and the archive format
  // would otherwise record it anyway as the owner of each file in it.
  it('should not record who owns the archived files', async () => {
    samples.setServiceInfo('core', 'stdOut', 'log line\n');

    const archivePath = await archiveSamples(samples, archiveRoot, config);

    const owners = [];

    await list({
      file: archivePath,
      onReadEntry: (entry) => owners.push([
        entry.header.uname || null,
        entry.header.gname || null,
        entry.header.uid ?? null,
        entry.header.gid ?? null,
      ]),
    });

    expect(owners).to.not.be.empty();
    expect(owners).to.deep.equal(owners.map(() => [null, null, null, null]));
  });

  // The archive is written into the operator's working directory, apart from
  // the temp directory the samples are staged in
  describe('archive file', () => {
    let outputDir;

    beforeEach(() => {
      outputDir = path.join(archiveRoot, 'output');
      fs.mkdirSync(outputDir);

      samples.date = new Date('2026-11-26T10:15:30.123Z');
      samples.setDashmateVersion('5.2.0');
    });

    // Operators run doctor on Windows too, where a file name cannot contain ':'
    it('should name the archive with a timestamp that is a valid file name everywhere', async () => {
      const archiveFilePath = await archiveSamples(samples, outputDir, config);

      const archiveName = 'dashmate-report-2026-11-26T10-15-30.123Z.tar.gz';

      expect(fs.readdirSync(outputDir)).to.deep.equal([archiveName]);
      expect(archiveFilePath).to.equal(path.join(outputDir, archiveName));
    });

    // The file name is only a label: the collection time an analysis judges
    // certificate dates against is the one recorded inside the archive
    it('should keep the exact collection time inside the archive', async () => {
      await archiveSamples(samples, outputDir, config);

      const [archiveName] = fs.readdirSync(outputDir);
      const unarchived = await unarchiveSamplesFactory(() => [])(path.join(outputDir, archiveName));

      expect(unarchived.date.toISOString()).to.equal('2026-11-26T10:15:30.123Z');
    });

    // The archive holds diagnostics about the node, and it is written under a
    // temporary name first: if renaming and removing it both fail, it stays
    // behind, so it is created readable by its owner only
    it('should create the archive accessible to its owner only', async function it() {
      if (process.platform === 'win32') {
        // Windows does not express access through file modes
        this.skip();
      }

      // A umask that leaves files readable by others, as most systems have
      const previousUmask = process.umask(0o022);

      let archiveFilePath;
      try {
        archiveFilePath = await archiveSamples(samples, outputDir, config);
      } finally {
        process.umask(previousUmask);
      }

      // eslint-disable-next-line no-bitwise
      expect(fs.statSync(archiveFilePath).mode & 0o777).to.equal(0o600);
    });

    // Packing succeeded, but the archive could not be put under its name
    it('should not leave the packed archive behind when it cannot be renamed into place', async () => {
      fs.mkdirSync(path.join(outputDir, 'dashmate-report-2026-11-26T10-15-30.123Z.tar.gz'));

      const error = await archiveSamples(samples, outputDir, config).catch((e) => e);

      expect(error).to.be.an('error');
      expect(error.syscall).to.equal('rename');
      expect(fs.readdirSync(outputDir).filter((name) => name.endsWith('.partial'))).to.be.empty();
    });

    // A temporary name already taken belongs to someone else
    it('should not remove a temporary file it did not create', async function it() {
      this.sinon.stub(crypto, 'randomBytes').returns(Buffer.from('0badf00d', 'hex'));

      const takenPath = path.join(outputDir, 'dashmate-report-2026-11-26T10-15-30.123Z.tar.gz.0badf00d.partial');
      fs.writeFileSync(takenPath, 'not ours');

      await expect(archiveSamples(samples, outputDir, config)).to.be.rejectedWith('EEXIST');

      expect(fs.readFileSync(takenPath, 'utf8')).to.equal('not ours');
    });

    // Once the archive file is open, only a finished or failed pack closes
    // it. A failure before packing starts, such as listing the staged
    // samples, must not leave the file open or behind.
    it('should not open the archive file when the staged samples cannot be listed', async function it() {
      const { readdirSync } = fs;
      this.sinon.stub(fs, 'readdirSync').callsFake((dirPath, ...args) => {
        if (path.dirname(dirPath) === archiveRoot && path.basename(dirPath).startsWith('dashmate-report-')) {
          throw Object.assign(new Error('EACCES: permission denied'), { code: 'EACCES' });
        }

        return readdirSync(dirPath, ...args);
      });
      const openSync = this.sinon.spy(fs, 'openSync');

      await expect(archiveSamples(samples, outputDir, config)).to.be.rejectedWith('EACCES');

      const partialOpens = openSync.getCalls().filter((call) => String(call.args[0]).endsWith('.partial'));

      expect(partialOpens).to.be.empty();
      expect(fs.readdirSync(outputDir)).to.be.empty();
      expect(fs.readdirSync(archiveRoot)).to.deep.equal(['output']);
    });

    describe('when packing fails', () => {
      beforeEach(function beforeEach() {
        // Root reads a file whatever its mode, and Windows cannot revoke read
        // access with a mode, so packing cannot be made to fail there
        if (process.getuid?.() === 0 || process.platform === 'win32') {
          this.skip();
        }

        // Make one staged sample unreadable, so that packing fails after the
        // archive file has been opened
        const { writeFileSync } = fs;
        this.sinon.stub(fs, 'writeFileSync').callsFake((filePath, ...args) => {
          writeFileSync(filePath, ...args);

          if (path.basename(filePath) === 'dashmateVersion.txt') {
            fs.chmodSync(filePath, 0);
          }
        });
      });

      // A truncated archive looks like a report to whoever is sent it, and
      // doctor cannot analyse it
      it('should not leave a partial archive behind', async () => {
        await expect(archiveSamples(samples, outputDir, config)).to.be.rejectedWith('EACCES');

        expect(fs.readdirSync(outputDir)).to.be.empty();
      });

      // A report already at that path was not written by this run
      it('should keep a report that was already at the destination', async () => {
        const existingReport = path.join(outputDir, 'dashmate-report-2026-11-26T10-15-30.123Z.tar.gz');
        fs.writeFileSync(existingReport, 'previous report');

        await expect(archiveSamples(samples, outputDir, config)).to.be.rejectedWith('EACCES');

        expect(fs.readFileSync(existingReport, 'utf8')).to.equal('previous report');
      });

      // The packing failure is what the operator has to act on
      it('should report the packing error when the cleanup fails too', async function it() {
        const { rmSync } = fs;
        this.sinon.stub(fs, 'rmSync').callsFake((filePath, ...args) => {
          if (filePath.startsWith(outputDir)) {
            throw Object.assign(new Error('EPERM: operation not permitted'), { code: 'EPERM' });
          }

          return rmSync(filePath, ...args);
        });

        await expect(archiveSamples(samples, outputDir, config)).to.be.rejectedWith('EACCES');
      });

      // Removing the staged samples can fail as well, and the packing failure
      // is still what the operator has to act on
      it('should report the packing error when removing the staged files fails too', async function it() {
        const { rmSync } = fs;
        this.sinon.stub(fs, 'rmSync').callsFake((filePath, ...args) => {
          if (path.dirname(filePath) === archiveRoot && path.basename(filePath).startsWith('dashmate-report-')) {
            throw Object.assign(new Error('EPERM: operation not permitted'), { code: 'EPERM' });
          }

          return rmSync(filePath, ...args);
        });

        await expect(archiveSamples(samples, outputDir, config)).to.be.rejectedWith('EACCES');
      });
    });
  });
});
