import fs from 'fs';
import { spawn } from 'child_process';
import { expect } from 'chai';
import HomeDir from '../../../../src/config/HomeDir.js';
import getBaseConfigFactory from '../../../../configs/defaults/getBaseConfigFactory.js';
import ConfigFile from '../../../../src/config/configFile/ConfigFile.js';
import ConfigFileJsonRepository from '../../../../src/config/configFile/ConfigFileJsonRepository.js';

const CURRENT_FORMAT_VERSION = '4.1.0';

describe('ConfigFileJsonRepository', () => {
  let homeDir;
  let configFilePath;
  let identityMigration;

  /**
   * Seed a valid config file on disk, the way a configured node would have one.
   *
   * @return {string} the JSON written
   */
  function seedConfigFile() {
    const baseConfig = getBaseConfigFactory(homeDir)();

    const configFile = new ConfigFile(
      [baseConfig],
      CURRENT_FORMAT_VERSION,
      'abcdef12',
      'base',
      null,
    );

    const json = `${JSON.stringify(configFile.toObject(), undefined, 2)}\n`;

    fs.writeFileSync(configFilePath, json, 'utf8');

    return json;
  }

  beforeEach(() => {
    homeDir = HomeDir.createTemp();
    configFilePath = homeDir.joinPath('config.json');
    identityMigration = (data) => data;
  });

  afterEach(() => {
    homeDir.remove();
  });

  describe('#read', () => {
    // A load is not an edit. If hydration leaves the file dirty, BaseCommand
    // persists it on exit even for `config get`, which is what lets a slow
    // reader overwrite a concurrent `config set`.
    it('should return a clean config file when nothing was migrated', () => {
      seedConfigFile();

      const repository = new ConfigFileJsonRepository(identityMigration, homeDir);

      const configFile = repository.read();

      expect(configFile.isChanged()).to.be.false();
      expect(configFile.getAllConfigs().filter((config) => config.isChanged())).to.be.empty();
    });

    it('should return a changed config file when it was migrated', () => {
      seedConfigFile();

      const migration = (data) => ({ ...data, configFormatVersion: '9.9.9' });

      const repository = new ConfigFileJsonRepository(migration, homeDir);

      const configFile = repository.read();

      expect(configFile.isChanged()).to.be.true();
      expect(configFile.getAllConfigs().every((config) => config.isChanged())).to.be.true();
    });

    // The symptom operators reported. Asserting on file CONTENT would pass
    // against the buggy code, because re-serializing an unchanged config is
    // byte-identical - the defect is that the write happens at all, carrying a
    // stale snapshot.
    it('should leave the file untouched when a read-only command exits', () => {
      seedConfigFile();

      const repository = new ConfigFileJsonRepository(identityMigration, homeDir);
      const { mtimeMs, ino } = fs.statSync(configFilePath);

      const configFile = repository.read();

      // What BaseCommand.finally() does
      if (configFile.isChanged()) {
        repository.write(configFile);
      }

      const after = fs.statSync(configFilePath);

      expect(after.mtimeMs).to.equal(mtimeMs);
      expect(after.ino).to.equal(ino);
    });
  });

  describe('#write', () => {
    it('should persist changes and mark the config file and its configs saved', () => {
      seedConfigFile();

      const repository = new ConfigFileJsonRepository(identityMigration, homeDir);
      const configFile = repository.read();

      configFile.getConfig('base').set('description', 'changed');

      repository.write(configFile);

      expect(configFile.isChanged()).to.be.false();
      expect(configFile.getAllConfigs().filter((config) => config.isChanged())).to.be.empty();

      const reread = new ConfigFileJsonRepository(identityMigration, homeDir).read();

      expect(reread.getConfig('base').get('description')).to.equal('changed');
    });

    it('should preserve the file mode, valid JSON and trailing newline, and leave no temp files', () => {
      seedConfigFile();
      fs.chmodSync(configFilePath, 0o600);

      const repository = new ConfigFileJsonRepository(identityMigration, homeDir);
      const configFile = repository.read();

      configFile.getConfig('base').set('description', 'changed');

      repository.write(configFile);

      const raw = fs.readFileSync(configFilePath, 'utf8');

      // eslint-disable-next-line no-bitwise
      expect(fs.statSync(configFilePath).mode & 0o777).to.equal(0o600);
      expect(raw.endsWith('}\n')).to.be.true();
      expect(() => JSON.parse(raw)).to.not.throw();
      expect(fs.readdirSync(homeDir.getPath()).filter((n) => n !== 'config.json')).to.be.empty();
    });

    // The helper daemon reads once at startup and writes on every certificate
    // renewal for the life of the process. If the staleness baseline is not
    // refreshed after each write, its second write conflicts with its own first.
    it('should allow repeated writes from one long-lived instance', () => {
      seedConfigFile();

      const repository = new ConfigFileJsonRepository(identityMigration, homeDir);
      const configFile = repository.read();

      configFile.getConfig('base').set('description', 'first');
      repository.write(configFile);

      configFile.getConfig('base').set('description', 'second');

      expect(() => repository.write(configFile)).to.not.throw();

      const reread = new ConfigFileJsonRepository(identityMigration, homeDir).read();

      expect(reread.getConfig('base').get('description')).to.equal('second');
    });
  });

  describe('#write concurrency', () => {
    // The reported lost update: a process loads the config, another process
    // changes and saves it, and the first process writes its stale snapshot
    // last. Asserting only "the second value survived" would also pass if the
    // stale write silently did nothing, so the contract itself is pinned:
    // it refuses loudly, leaves the file alone, and keeps the rejected state.
    it('should refuse a stale write instead of clobbering a concurrent update', () => {
      seedConfigFile();

      const slowReader = new ConfigFileJsonRepository(identityMigration, homeDir);
      const staleConfigFile = slowReader.read();

      // A concurrent `dashmate config set` lands and exits successfully
      const writer = new ConfigFileJsonRepository(identityMigration, homeDir);
      const freshConfigFile = writer.read();
      freshConfigFile.getConfig('base').set('description', 'set-by-concurrent-writer');
      writer.write(freshConfigFile);

      const onDisk = fs.readFileSync(configFilePath, 'utf8');

      staleConfigFile.getConfig('base').set('description', 'stale-snapshot');

      let thrown;
      try {
        slowReader.write(staleConfigFile);
      } catch (e) {
        thrown = e;
      }

      expect(thrown, 'stale write must not succeed').to.exist();
      expect(thrown.code).to.equal('DASHMATE_CONFIG_FILE_CONFLICT');

      // the concurrent writer's value survives, byte for byte
      expect(fs.readFileSync(configFilePath, 'utf8')).to.equal(onDisk);

      // nothing generated in memory is lost - it is parked for the operator
      const rejected = fs.readdirSync(homeDir.getPath())
        .filter((name) => name.startsWith('config.json.rejected-'));

      expect(rejected).to.have.lengthOf(1);
      expect(thrown.message).to.contain(rejected[0]);

      const parked = JSON.parse(fs.readFileSync(homeDir.joinPath(rejected[0]), 'utf8'));

      expect(parked.configs.base.description).to.equal('stale-snapshot');
    });

    // Two nodes being set up at once both find no config file. "I have never
    // looked" and "I looked and there was nothing there" are different claims,
    // and only the second can detect that someone else created the file first.
    it('should refuse a first-run write when another process created the file first', () => {
      const first = new ConfigFileJsonRepository(identityMigration, homeDir);
      const second = new ConfigFileJsonRepository(identityMigration, homeDir);

      // both observe an absent config file
      expect(() => first.read()).to.throw();
      expect(() => second.read()).to.throw();

      const configFile = new ConfigFile(
        [getBaseConfigFactory(homeDir)()],
        CURRENT_FORMAT_VERSION,
        'abcdef12',
        'base',
        null,
      );

      first.write(configFile);

      const onDisk = fs.readFileSync(configFilePath, 'utf8');

      let thrown;
      try {
        second.write(configFile);
      } catch (e) {
        thrown = e;
      }

      expect(thrown, 'second first-run write must not clobber the first').to.exist();
      expect(thrown.code).to.equal('DASHMATE_CONFIG_FILE_CONFLICT');
      expect(fs.readFileSync(configFilePath, 'utf8')).to.equal(onDisk);
    });

    // Present-to-absent is a change like any other. Recreating our old snapshot
    // over a deliberate removal is the same lost update in the other direction.
    it('should refuse to recreate a config file that was deleted after it was read', () => {
      seedConfigFile();

      const repository = new ConfigFileJsonRepository(identityMigration, homeDir);
      const configFile = repository.read();

      fs.unlinkSync(configFilePath);

      configFile.getConfig('base').set('description', 'stale-snapshot');

      let thrown;
      try {
        repository.write(configFile);
      } catch (e) {
        thrown = e;
      }

      expect(thrown, 'must not resurrect a deleted config file').to.exist();
      expect(thrown.code).to.equal('DASHMATE_CONFIG_FILE_CONFLICT');
      expect(fs.existsSync(configFilePath)).to.be.false();
    });

    // Comparing against the baseline is not enough on its own: two writers can
    // both read the same bytes, both decide they are current, and both replace
    // the file. Only a lock closes that window, and only a genuinely separate
    // process can prove the lock is honoured. proper-lockfile represents a held
    // lock as a directory, so the other process needs nothing but `fs`.
    it('should wait for a lock held by another process before writing', function it0(done) {
      this.timeout(20000);

      seedConfigFile();

      const repository = new ConfigFileJsonRepository(identityMigration, homeDir);
      const configFile = repository.read();

      configFile.getConfig('base').set('description', 'written-after-waiting');

      const lockPath = homeDir.joinPath('config.json.lock');
      const holdMs = 700;

      const child = spawn(
        process.execPath,
        ['-e', `require('fs').mkdirSync(${JSON.stringify(lockPath)});`
          + `setTimeout(() => require('fs').rmdirSync(${JSON.stringify(lockPath)}), ${holdMs});`],
        { stdio: 'ignore' },
      );

      child.on('error', done);

      // Wait for the other process to actually take the lock. Synchronous on
      // purpose - the write we are about to make is synchronous too.
      const deadline = Date.now() + 10000;
      while (!fs.existsSync(lockPath) && Date.now() < deadline) {
        // busy-wait
      }

      expect(fs.existsSync(lockPath), 'other process should hold the lock').to.be.true();

      const startedAt = Date.now();

      repository.write(configFile);

      const waitedMs = Date.now() - startedAt;

      // It blocked rather than writing straight through the other holder.
      expect(waitedMs).to.be.at.least(holdMs / 2);
      expect(fs.existsSync(lockPath), 'lock should be released again').to.be.false();

      const reread = new ConfigFileJsonRepository(identityMigration, homeDir).read();

      expect(reread.getConfig('base').get('description')).to.equal('written-after-waiting');

      child.on('exit', () => done());
    });

    it('should succeed after reloading following a conflict', () => {
      seedConfigFile();

      const slowReader = new ConfigFileJsonRepository(identityMigration, homeDir);
      slowReader.read();

      const writer = new ConfigFileJsonRepository(identityMigration, homeDir);
      const freshConfigFile = writer.read();
      freshConfigFile.getConfig('base').set('description', 'concurrent');
      writer.write(freshConfigFile);

      const reloaded = slowReader.read();
      reloaded.getConfig('base').set('description', 'after-reload');

      expect(() => slowReader.write(reloaded)).to.not.throw();

      const reread = new ConfigFileJsonRepository(identityMigration, homeDir).read();

      expect(reread.getConfig('base').get('description')).to.equal('after-reload');
    });
  });
});
