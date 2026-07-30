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
  // Tracked so a failed assertion cannot leave a spawned lock holder behind
  // still owning the lock directory after the test that created it is gone.
  let lockHolder;

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
    if (lockHolder && lockHolder.exitCode === null && lockHolder.signalCode === null) {
      lockHolder.kill();
    }

    lockHolder = undefined;

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
    // renewal for the life of the process, so saving repeatedly from one
    // instance has to keep working.
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

  describe('#update', () => {
    // The reported bug, stated as the outcome operators want rather than as a
    // failure they have to recover from: one process loads the config, another
    // changes and saves a different option, and both edits survive. Mutating a
    // snapshot loaded earlier and saving it at exit loses one of them.
    it('should keep concurrent edits to different options', () => {
      seedConfigFile();

      const slowCommand = new ConfigFileJsonRepository(identityMigration, homeDir);
      const otherCommand = new ConfigFileJsonRepository(identityMigration, homeDir);

      // Both load the same starting state, as two overlapping commands would
      slowCommand.read();
      otherCommand.read();

      otherCommand.update((configFile) => {
        configFile.getConfig('base').set('description', 'set-by-other-command');
      });

      slowCommand.update((configFile) => {
        configFile.getConfig('base').set('core.rpc.port', 30000);
      });

      const reread = new ConfigFileJsonRepository(identityMigration, homeDir).read();

      expect(reread.getConfig('base').get('description')).to.equal('set-by-other-command');
      expect(reread.getConfig('base').get('core.rpc.port')).to.equal(30000);
    });

    // The mutator must never be handed the state this instance happened to read
    // earlier - that is the whole point of reading inside the lock.
    it('should hand the mutator state written since this instance last read', () => {
      seedConfigFile();

      const repository = new ConfigFileJsonRepository(identityMigration, homeDir);

      repository.read();

      new ConfigFileJsonRepository(identityMigration, homeDir)
        .update((configFile) => {
          configFile.getConfig('base').set('description', 'written-by-someone-else');
        });

      let seen;
      repository.update((configFile) => {
        seen = configFile.getConfig('base').get('description');
      });

      expect(seen).to.equal('written-by-someone-else');
    });

    it('should return the state it saved', () => {
      seedConfigFile();

      const repository = new ConfigFileJsonRepository(identityMigration, homeDir);

      const configFile = repository.update((freshConfigFile) => {
        freshConfigFile.getConfig('base').set('description', 'returned');
      });

      expect(configFile.getConfig('base').get('description')).to.equal('returned');
      expect(configFile.isChanged()).to.be.false();
    });

    it('should not write when the mutator throws', () => {
      const before = seedConfigFile();

      const repository = new ConfigFileJsonRepository(identityMigration, homeDir);

      expect(() => repository.update(() => {
        throw new Error('mutator failed');
      })).to.throw('mutator failed');

      expect(fs.readFileSync(configFilePath, 'utf8')).to.equal(before);
      expect(fs.existsSync(homeDir.joinPath('config.json.lock'))).to.be.false();
    });

    // Reading fresh inside the lock is only sound if the lock is actually
    // honoured across processes. proper-lockfile represents a held lock as a
    // directory, so another process needs nothing but `fs` to hold it.
    it('should wait for a lock held by another process', function it0(done) {
      this.timeout(20000);

      seedConfigFile();

      const repository = new ConfigFileJsonRepository(identityMigration, homeDir);
      const lockPath = homeDir.joinPath('config.json.lock');
      const holdMs = 700;

      lockHolder = spawn(
        process.execPath,
        ['-e', `require('fs').mkdirSync(${JSON.stringify(lockPath)});`
          + `setTimeout(() => require('fs').rmdirSync(${JSON.stringify(lockPath)}), ${holdMs});`],
        { stdio: 'ignore' },
      );

      lockHolder.on('error', done);

      const deadline = Date.now() + 10000;
      while (!fs.existsSync(lockPath) && Date.now() < deadline) {
        Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0, 10);
      }

      expect(fs.existsSync(lockPath), 'other process should hold the lock').to.be.true();

      const startedAt = Date.now();

      repository.update((configFile) => {
        configFile.getConfig('base').set('description', 'written-after-waiting');
      });

      expect(Date.now() - startedAt).to.be.at.least(holdMs / 2);

      const reread = new ConfigFileJsonRepository(identityMigration, homeDir).read();

      expect(reread.getConfig('base').get('description')).to.equal('written-after-waiting');

      lockHolder.on('exit', () => done());
    });
  });
});
