import fs from 'fs';
import path from 'path';
import { spawn } from 'child_process';
import { expect } from 'chai';
import HomeDir from '../../../../src/config/HomeDir.js';
import getBaseConfigFactory from '../../../../configs/defaults/getBaseConfigFactory.js';
import ConfigFile from '../../../../src/config/configFile/ConfigFile.js';
import ConfigFileJsonRepository from '../../../../src/config/configFile/ConfigFileJsonRepository.js';
import ConfigFileMigrationRequiredError from '../../../../src/config/errors/ConfigFileMigrationRequiredError.js';
import ConfigFileNotFoundError from '../../../../src/config/errors/ConfigFileNotFoundError.js';
import InvalidConfigFileFormatError from '../../../../src/config/errors/InvalidConfigFileFormatError.js';
import createDIContainer from '../../../../src/createDIContainer.js';
import getConfigFileDataV0250 from '../../../../src/test/fixtures/getConfigFileDataV0250.js';

const CURRENT_FORMAT_VERSION = '4.1.0';

describe('ConfigFileJsonRepository', () => {
  let homeDir;
  let configFilePath;
  let identityMigration;
  let createDefaults;
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
    createDefaults = () => {
      const configFile = new ConfigFile(
        [getBaseConfigFactory(homeDir)()],
        CURRENT_FORMAT_VERSION,
        'abcdef12',
        'base',
        null,
      );

      configFile.markAsChanged();

      return configFile;
    };
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

      const repository = new ConfigFileJsonRepository(identityMigration, homeDir, createDefaults);

      const configFile = repository.read();

      expect(configFile.isChanged()).to.be.false();
      expect(configFile.getAllConfigs().filter((config) => config.isChanged())).to.be.empty();
    });

    it('should return a changed config file when it was migrated', () => {
      seedConfigFile();

      const migration = (data) => ({ ...data, configFormatVersion: '9.9.9' });

      const repository = new ConfigFileJsonRepository(migration, homeDir, createDefaults);

      const configFile = repository.read();

      expect(configFile.isChanged()).to.be.true();
      expect(configFile.getAllConfigs().every((config) => config.isChanged())).to.be.true();
    });

    it('should not require a write after an unchanged read', () => {
      seedConfigFile();

      const repository = new ConfigFileJsonRepository(identityMigration, homeDir, createDefaults);
      const { mtimeMs, ino } = fs.statSync(configFilePath);

      const configFile = repository.read();

      if (configFile.isChanged()) {
        repository.write(configFile);
      }

      const after = fs.statSync(configFilePath);

      expect(after.mtimeMs).to.equal(mtimeMs);
      expect(after.ino).to.equal(ino);
    });

    // A name Dashmate would refuse to create today may already be in a config
    // file written before the rule existed. Refusing to load the collection
    // would leave no way to run the command that removes the entry.
    it('should still load a persisted config whose name would be refused today', () => {
      const repository = new ConfigFileJsonRepository(identityMigration, homeDir, createDefaults);

      for (const name of ['config.json.lock', 'CONFIG.JSON', 'config.json.']) {
        seedConfigFile();

        const data = JSON.parse(fs.readFileSync(configFilePath, 'utf8'));
        data.configs[name] = data.configs.base;
        fs.writeFileSync(configFilePath, `${JSON.stringify(data, undefined, 2)}\n`, 'utf8');

        expect(() => repository.read(), name).to.not.throw();
        expect(repository.read().isConfigExists(name), name).to.be.true();
      }
    });

    it('should scope a validation bypass to the configs selected by its predicate', () => {
      seedConfigFile();

      const data = JSON.parse(fs.readFileSync(configFilePath, 'utf8'));
      const validNetwork = data.configs.base.network;
      data.configs.base.network = 'invalid';
      data.configs.node1 = { ...data.configs.base, network: validNetwork };
      fs.writeFileSync(configFilePath, `${JSON.stringify(data, undefined, 2)}\n`, 'utf8');

      const repository = new ConfigFileJsonRepository(identityMigration, homeDir, createDefaults);
      const skipBaseValidation = ({ name }) => name === 'base';
      const configFile = repository.read({ skipValidation: skipBaseValidation });

      expect(configFile.getConfig('base').getStored('network')).to.equal('invalid');

      data.configs.node1.network = 'invalid';
      fs.writeFileSync(configFilePath, `${JSON.stringify(data, undefined, 2)}\n`, 'utf8');

      expect(() => repository.read({ skipValidation: skipBaseValidation }))
        .to.throw('network must be equal to one of the allowed values');
    });
  });

  describe('#write', () => {
    it('should persist changes while keeping configs dirty until templates are rendered', () => {
      seedConfigFile();

      const repository = new ConfigFileJsonRepository(identityMigration, homeDir, createDefaults);
      const configFile = repository.read();

      configFile.getConfig('base').set('description', 'changed');

      repository.write(configFile);

      expect(configFile.changed).to.be.false();
      expect(configFile.getConfig('base').isChanged()).to.be.true();

      const reread = new ConfigFileJsonRepository(identityMigration, homeDir, createDefaults).read();

      expect(reread.getConfig('base').get('description')).to.equal('changed');
    });

    it('should preserve the file mode, valid JSON and trailing newline, and leave no temp files', () => {
      seedConfigFile();
      fs.chmodSync(configFilePath, 0o600);

      const repository = new ConfigFileJsonRepository(identityMigration, homeDir, createDefaults);
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

    // A caller holding the command-length lock may save multiple checkpoints
    // through the same repository instance.
    it('should allow repeated writes from one long-lived instance', () => {
      seedConfigFile();

      const repository = new ConfigFileJsonRepository(identityMigration, homeDir, createDefaults);
      const configFile = repository.read();

      configFile.getConfig('base').set('description', 'first');
      repository.write(configFile);

      configFile.getConfig('base').set('description', 'second');

      expect(() => repository.write(configFile)).to.not.throw();

      const reread = new ConfigFileJsonRepository(identityMigration, homeDir, createDefaults).read();

      expect(reread.getConfig('base').get('description')).to.equal('second');
    });
  });

  describe('#update', () => {
    it('should render and save a migration while holding the lock', () => {
      seedConfigFile();

      const migration = (data) => ({ ...data, configFormatVersion: '9.9.9' });
      const repository = new ConfigFileJsonRepository(migration, homeDir, createDefaults);
      let renderedWhileLocked = false;

      let migrated;
      const result = repository.readAndMigrate({}, (migratedConfigs) => {
        migrated = migratedConfigs;
        renderedWhileLocked = fs.existsSync(homeDir.joinPath('.config.json.lock'));
        expect(JSON.parse(fs.readFileSync(configFilePath, 'utf8')).configFormatVersion)
          .to.equal(CURRENT_FORMAT_VERSION);
      });
      const { configFile } = result;

      expect(configFile.getConfigFormatVersion()).to.equal('9.9.9');
      expect(migrated).to.have.length(1);
      expect(result).to.not.have.property('migrated');
      expect(renderedWhileLocked).to.be.true();
      expect(JSON.parse(fs.readFileSync(configFilePath, 'utf8')).configFormatVersion)
        .to.equal('9.9.9');
    });

    // A command that promises to change nothing must keep that promise even on
    // the one run where a migration is due. Migrations are not all pure - one
    // copies TLS files to a new location and removes the originals, and then
    // deletes the whole ssl directory - so migrating on its behalf would move
    // and delete files outside any lock, from a command documented as safe to
    // run against a node that is still up.
    it('should refuse to migrate for a caller that changes nothing', () => {
      seedConfigFile();

      const seeded = JSON.parse(seedConfigFile());
      seeded.configFormatVersion = '0.25.0';
      fs.writeFileSync(configFilePath, JSON.stringify(seeded, undefined, 2), 'utf8');

      let migrationRuns = 0;
      const migration = (data) => {
        migrationRuns += 1;

        return { ...data, configFormatVersion: CURRENT_FORMAT_VERSION };
      };
      const repository = new ConfigFileJsonRepository(
        migration,
        homeDir,
        createDefaults,
        CURRENT_FORMAT_VERSION,
      );
      const before = fs.readFileSync(configFilePath, 'utf8');

      expect(() => repository.readAndMigrate({ readOnly: true }))
        .to.throw(ConfigFileMigrationRequiredError);

      // Not "migrated in memory and discarded" - not run at all, because
      // running it is what touches the disk.
      expect(migrationRuns).to.equal(0);
      expect(fs.readFileSync(configFilePath, 'utf8')).to.equal(before);
      expect(fs.existsSync(homeDir.joinPath('.config.json.lock'))).to.be.false();
    });

    // Refusing has to mean "the recorded version is genuinely behind", not
    // "something about this file defeated the probe". A missing or damaged
    // config file has its own errors, and one of them is what first-run setup
    // catches to create defaults - reporting a migration instead breaks that
    // and tells the operator something untrue about a file that may not exist.
    it('should let a missing config file report itself', () => {
      const repository = new ConfigFileJsonRepository(
        identityMigration,
        homeDir,
        createDefaults,
        CURRENT_FORMAT_VERSION,
      );

      expect(() => repository.readAndMigrate({ readOnly: true }))
        .to.throw(ConfigFileNotFoundError);
    });

    it('should let a malformed config file report itself', () => {
      fs.writeFileSync(configFilePath, '{ not json at all', 'utf8');

      const repository = new ConfigFileJsonRepository(
        identityMigration,
        homeDir,
        createDefaults,
        CURRENT_FORMAT_VERSION,
      );

      expect(() => repository.readAndMigrate({ readOnly: true }))
        .to.throw(InvalidConfigFileFormatError);
    });

    [
      ['no recorded version', ({ configFormatVersion, ...rest }) => rest],
      ['an unparseable recorded version', (data) => ({ ...data, configFormatVersion: 'not-a-version' })],
    ].forEach(([name, damage]) => {
      it(`should not claim a migration is due from ${name}`, () => {
        const damaged = damage(JSON.parse(seedConfigFile()));
        fs.writeFileSync(configFilePath, JSON.stringify(damaged, undefined, 2), 'utf8');

        const repository = new ConfigFileJsonRepository(
          identityMigration,
          homeDir,
          createDefaults,
          CURRENT_FORMAT_VERSION,
        );

        // Whatever this file's own problem turns out to be, it is not that an
        // older dashmate wrote it - nothing here establishes that.
        expect(() => repository.readAndMigrate({ readOnly: true }))
          .to.not.throw(ConfigFileMigrationRequiredError);
      });
    });

    // Falling through to read() on an undetermined version is only safe because
    // the version comparison inside the migration chain rejects a version it
    // cannot parse before any migration body runs. Proven against the shipped
    // migrations rather than argued, because one of those bodies deletes a
    // directory of TLS material.
    [
      ['no recorded version', ({ configFormatVersion, ...rest }) => rest],
      ['an unparseable recorded version', (data) => ({ ...data, configFormatVersion: 'bad' })],
    ].forEach(([name, damage]) => {
      it(`should run no migration for a read-only caller given ${name}`, async () => {
        const container = await createDIContainer();
        container.resolve('homeDir').change(homeDir);

        const legacy = damage(getConfigFileDataV0250());
        const [legacyName] = Object.keys(legacy.configs);
        fs.writeFileSync(configFilePath, JSON.stringify(legacy, undefined, 2), 'utf8');

        const legacySslDir = homeDir.joinPath('ssl', legacyName);
        fs.mkdirSync(legacySslDir, { recursive: true });
        fs.writeFileSync(path.join(legacySslDir, 'bundle.crt'), 'certificate', 'utf8');

        const repository = new ConfigFileJsonRepository(
          container.resolve('migrateConfigFile'),
          homeDir,
          createDefaults,
          container.resolve('configFormatVersion'),
        );

        expect(() => repository.readAndMigrate({ readOnly: true })).to.throw();

        expect(fs.existsSync(path.join(legacySslDir, 'bundle.crt'))).to.be.true();
        expect(fs.existsSync(homeDir.joinPath('ssl'))).to.be.true();
        expect(fs.existsSync(homeDir.joinPath('.config.json.lock'))).to.be.false();
      });
    });

    // The run this mode exists for is the first one after an upgrade, which is
    // exactly when a migration is due. Refusing there fails the operator at the
    // moment they were told the command was safe - before they stop a healthy
    // node. Almost every migration only reshapes data, so it can be applied in
    // memory and thrown away.
    it('should migrate in memory when no migration touches the disk', async () => {
      const container = await createDIContainer();
      container.resolve('homeDir').change(homeDir);

      const configFormatVersion = container.resolve('configFormatVersion');

      const seeded = JSON.parse(seedConfigFile());
      seeded.configFormatVersion = '4.1.0';
      fs.writeFileSync(configFilePath, JSON.stringify(seeded, undefined, 2), 'utf8');

      const before = fs.readFileSync(configFilePath, 'utf8');

      const repository = new ConfigFileJsonRepository(
        container.resolve('migrateConfigFile'),
        homeDir,
        createDefaults,
        configFormatVersion,
      );

      let rendered = false;
      const { configFile } = repository.readAndMigrate(
        { readOnly: true },
        () => { rendered = true; },
      );

      // The caller gets current data to judge, and the disk is untouched.
      expect(configFile.getConfigFormatVersion()).to.equal(configFormatVersion);
      expect(rendered).to.be.false();
      expect(fs.readFileSync(configFilePath, 'utf8')).to.equal(before);
      expect(fs.existsSync(homeDir.joinPath('.config.json.lock'))).to.be.false();
    });

    // The two migrations that move and delete TLS material are the only reason
    // this mode ever refuses, so it has to refuse when one of them is in range.
    it('should still refuse when a migration in range touches the disk', async () => {
      const container = await createDIContainer();
      container.resolve('homeDir').change(homeDir);

      const legacy = getConfigFileDataV0250();
      fs.writeFileSync(configFilePath, JSON.stringify(legacy, undefined, 2), 'utf8');

      const repository = new ConfigFileJsonRepository(
        container.resolve('migrateConfigFile'),
        homeDir,
        createDefaults,
        container.resolve('configFormatVersion'),
      );

      expect(() => repository.readAndMigrate({ readOnly: true }))
        .to.throw(ConfigFileMigrationRequiredError);
    });

    // The common case, and the one that has to stay fast: nothing to migrate,
    // so nothing to refuse and no lock to take.
    it('should read without locking for a caller that changes nothing', () => {
      seedConfigFile();

      const repository = new ConfigFileJsonRepository(
        identityMigration,
        homeDir,
        createDefaults,
        CURRENT_FORMAT_VERSION,
      );
      const before = fs.readFileSync(configFilePath, 'utf8');

      const { configFile } = repository.readAndMigrate({ readOnly: true });

      expect(configFile.getConfig('base')).to.exist();
      expect(fs.readFileSync(configFilePath, 'utf8')).to.equal(before);
      expect(fs.existsSync(homeDir.joinPath('.config.json.lock'))).to.be.false();
    });

    // Every command dashmate prints falls back to the default node when it
    // carries no --config, and this error is raised from a layer that has no
    // idea which node was selected. So it names none: prose an operator cannot
    // paste at the wrong machine.
    it('should suggest no command it cannot aim at the right node', () => {
      const seeded = JSON.parse(seedConfigFile());
      seeded.configFormatVersion = '0.25.0';
      fs.writeFileSync(configFilePath, JSON.stringify(seeded, undefined, 2), 'utf8');

      const repository = new ConfigFileJsonRepository(
        identityMigration,
        homeDir,
        createDefaults,
        CURRENT_FORMAT_VERSION,
      );

      const error = (() => {
        try {
          repository.readAndMigrate({ readOnly: true });
        } catch (e) {
          return e;
        }

        return null;
      })();

      expect(error).to.be.an.instanceOf(ConfigFileMigrationRequiredError);
      expect(error.message).to.contain(configFilePath);

      // Prose may name dashmate; nothing may be laid out as a command to copy.
      error.message.split('\n').forEach((line) => {
        expect(line, line).to.not.match(/^\s+dashmate\s/);
        expect(line, line).to.not.match(/`dashmate\s/);
      });
    });

    // The migration this refuses to run really does delete things. Driven with
    // the shipped migration set rather than a stand-in, so the guard is pinned
    // against the behaviour it exists for and not against a mock of it.
    it('should leave the ssl directory alone that migrating would delete', async () => {
      const container = await createDIContainer();
      container.resolve('homeDir').change(homeDir);

      const migrateConfigFile = container.resolve('migrateConfigFile');
      const configFormatVersion = container.resolve('configFormatVersion');

      // A genuine config of that era, so the migrations that follow it run
      // against the shape they were written for.
      const legacy = getConfigFileDataV0250();
      const [legacyName] = Object.keys(legacy.configs);
      fs.writeFileSync(configFilePath, JSON.stringify(legacy, undefined, 2), 'utf8');

      const legacySslDir = homeDir.joinPath('ssl', legacyName);
      fs.mkdirSync(legacySslDir, { recursive: true });
      fs.writeFileSync(path.join(legacySslDir, 'bundle.crt'), 'certificate', 'utf8');

      const repository = new ConfigFileJsonRepository(
        migrateConfigFile,
        homeDir,
        createDefaults,
        configFormatVersion,
      );

      expect(() => repository.readAndMigrate({ readOnly: true }))
        .to.throw(ConfigFileMigrationRequiredError);

      expect(fs.existsSync(path.join(legacySslDir, 'bundle.crt'))).to.be.true();
      expect(fs.existsSync(homeDir.joinPath('ssl'))).to.be.true();
      expect(JSON.parse(fs.readFileSync(configFilePath, 'utf8')).configFormatVersion)
        .to.equal('0.25.0');

      // The control: a normal read migrates, and that is what removes them.
      repository.readAndMigrate();

      expect(fs.existsSync(homeDir.joinPath('ssl'))).to.be.false();
    });

    // Migrations are not all pure - one moves TLS files and deletes the
    // originals - so deciding whether one is due must not run them. Running
    // them to find out would do that work outside the lock, where another
    // command reconfiguring the node is free to be doing the same.
    it('should decide a migration is due without running any migration', () => {
      seedConfigFile();

      let migrationRuns = 0;
      const countingMigration = (data) => {
        migrationRuns += 1;

        return data;
      };

      const repository = new ConfigFileJsonRepository(
        countingMigration,
        homeDir,
        createDefaults,
        CURRENT_FORMAT_VERSION,
      );

      repository.readAndMigrate();

      // One read, inside no lock, because the recorded version is current.
      expect(migrationRuns).to.equal(1);
      expect(fs.existsSync(homeDir.joinPath('.config.json.lock'))).to.be.false();
    });

    it('should retry migration rendering when the first render fails', () => {
      seedConfigFile();

      const migration = (data) => ({ ...data, configFormatVersion: '9.9.9' });
      const repository = new ConfigFileJsonRepository(migration, homeDir, createDefaults);

      expect(() => repository.readAndMigrate({}, () => {
        throw new Error('template write failed');
      })).to.throw('template write failed');

      expect(JSON.parse(fs.readFileSync(configFilePath, 'utf8')).configFormatVersion)
        .to.equal(CURRENT_FORMAT_VERSION);

      let retried = false;
      repository.readAndMigrate({}, () => {
        retried = true;
      });

      expect(retried).to.be.true();
      expect(JSON.parse(fs.readFileSync(configFilePath, 'utf8')).configFormatVersion)
        .to.equal('9.9.9');
    });

    it('should not wait for a lock when reading does not migrate', () => {
      seedConfigFile();

      const repository = new ConfigFileJsonRepository(identityMigration, homeDir, createDefaults, CURRENT_FORMAT_VERSION, {
        acquireTimeout: 50,
      });
      const lockPath = homeDir.joinPath('.config.json.lock');

      fs.mkdirSync(lockPath);

      try {
        expect(() => repository.readAndMigrate()).to.not.throw();
      } finally {
        fs.rmdirSync(lockPath);
      }
    });

    it('should reuse a command-held lock when reading and saving a migration', () => {
      seedConfigFile();

      const migration = (data) => ({ ...data, configFormatVersion: '9.9.9' });
      const repository = new ConfigFileJsonRepository(migration, homeDir, createDefaults);
      let renderedWhileLocked = false;

      repository.acquire();

      try {
        repository.readAndMigrate({}, () => {
          renderedWhileLocked = fs.existsSync(homeDir.joinPath('.config.json.lock'));
        });
      } finally {
        repository.release();
      }

      expect(renderedWhileLocked).to.be.true();
    });

    // JSON is the authoritative state. If rendering fails after it is saved,
    // service files remain stale until the operator explicitly renders them.
    it('should keep the saved config when rendering service files fails', () => {
      seedConfigFile();

      const repository = new ConfigFileJsonRepository(identityMigration, homeDir, createDefaults);

      expect(() => repository.update((configFile) => {
        configFile.getConfig('base').set('description', 'saved-before-render');
      }, {
        onSaved: () => {
          throw new Error('template write failed');
        },
      })).to.throw('template write failed');

      expect(repository.read().getConfig('base').get('description')).to.equal('saved-before-render');
    });

    it('should render service files after the change reaches disk', () => {
      seedConfigFile();

      const repository = new ConfigFileJsonRepository(identityMigration, homeDir, createDefaults);

      let onDiskWhenRendering;

      repository.update((configFile) => {
        configFile.getConfig('base').set('description', 'saved-first');
      }, {
        onSaved: () => {
          onDiskWhenRendering = JSON.parse(fs.readFileSync(configFilePath, 'utf8'))
            .configs.base.description;
        },
      });

      expect(onDiskWhenRendering).to.equal('saved-first');
      expect(repository.read().getConfig('base').get('description')).to.equal('saved-first');
    });

    it('should keep repository locks outside the config-name namespace', () => {
      seedConfigFile();

      const repository = new ConfigFileJsonRepository(identityMigration, homeDir, createDefaults);

      repository.update((configFile) => {
        configFile.getConfig('base').set('description', 'changed');
      }, {
        onSaved: () => {
          expect(fs.existsSync(homeDir.joinPath('.config.json.lock'))).to.be.true();
          expect(fs.existsSync(homeDir.joinPath('config.json.lock'))).to.be.false();
        },
      });
    });

    it('should run onSaved after saving and while still holding the lock', () => {
      seedConfigFile();

      const repository = new ConfigFileJsonRepository(identityMigration, homeDir, createDefaults);

      let sawOnDisk;
      let heldLock;

      repository.update((configFile) => {
        configFile.getConfig('base').set('description', 'saved-then-observed');
      }, {
        onSaved: () => {
          sawOnDisk = JSON.parse(fs.readFileSync(configFilePath, 'utf8'))
            .configs.base.description;
          heldLock = fs.existsSync(homeDir.joinPath('.config.json.lock'));
        },
      });

      expect(sawOnDisk).to.equal('saved-then-observed');
      expect(heldLock, 'lock should still be held while onSaved runs').to.be.true();
    });

    it('should release the lock but keep the saved JSON when onSaved throws', () => {
      seedConfigFile();

      const repository = new ConfigFileJsonRepository(identityMigration, homeDir, createDefaults);

      expect(() => repository.update((configFile) => {
        configFile.getConfig('base').set('description', 'committed-before-hook');
      }, {
        onSaved: () => {
          throw new Error('template write failed');
        },
      })).to.throw('template write failed');

      expect(fs.existsSync(homeDir.joinPath('.config.json.lock'))).to.be.false();
      expect(repository.read().getConfig('base').get('description'))
        .to.equal('committed-before-hook');
    });

    // On a machine with no config file yet, the change still has to land -
    // reading first and failing would break `config create` on first run.
    it('should create the config file when there is none yet', () => {
      expect(fs.existsSync(configFilePath)).to.be.false();

      const repository = new ConfigFileJsonRepository(identityMigration, homeDir, createDefaults);

      repository.update((configFile) => {
        configFile.getConfig('base').set('description', 'created-on-first-run');
      });

      expect(fs.existsSync(configFilePath)).to.be.true();

      const reread = new ConfigFileJsonRepository(identityMigration, homeDir, createDefaults).read();

      expect(reread.getConfig('base').get('description')).to.equal('created-on-first-run');
    });

    // The reported bug, stated as the outcome operators want rather than as a
    // failure they have to recover from: one process loads the config, another
    // changes and saves a different option, and both edits survive. Mutating a
    // snapshot loaded earlier and saving it at exit loses one of them.
    it('should keep concurrent edits to different options', () => {
      seedConfigFile();

      const slowCommand = new ConfigFileJsonRepository(identityMigration, homeDir, createDefaults);
      const otherCommand = new ConfigFileJsonRepository(identityMigration, homeDir, createDefaults);

      // Both load the same starting state, as two overlapping commands would
      slowCommand.read();
      otherCommand.read();

      otherCommand.update((configFile) => {
        configFile.getConfig('base').set('description', 'set-by-other-command');
      });

      slowCommand.update((configFile) => {
        configFile.getConfig('base').set('core.rpc.port', 30000);
      });

      const reread = new ConfigFileJsonRepository(identityMigration, homeDir, createDefaults).read();

      expect(reread.getConfig('base').get('description')).to.equal('set-by-other-command');
      expect(reread.getConfig('base').get('core.rpc.port')).to.equal(30000);
    });

    // The mutator must never be handed the state this instance happened to read
    // earlier - that is the whole point of reading inside the lock.
    it('should hand the mutator state written since this instance last read', () => {
      seedConfigFile();

      const repository = new ConfigFileJsonRepository(identityMigration, homeDir, createDefaults);

      repository.read();

      new ConfigFileJsonRepository(identityMigration, homeDir, createDefaults)
        .update((configFile) => {
          configFile.getConfig('base').set('description', 'written-by-someone-else');
        });

      let seen;
      repository.update((configFile) => {
        seen = configFile.getConfig('base').get('description');
      });

      expect(seen).to.equal('written-by-someone-else');
    });

    it('should save the state with template work still marked pending', () => {
      seedConfigFile();

      const repository = new ConfigFileJsonRepository(identityMigration, homeDir, createDefaults);
      let savedConfigFile;

      const result = repository.update((freshConfigFile) => {
        freshConfigFile.getConfig('base').set('description', 'returned');
        savedConfigFile = freshConfigFile;
      });

      expect(result).to.be.undefined();
      expect(savedConfigFile.getConfig('base').get('description')).to.equal('returned');
      expect(savedConfigFile.changed).to.be.false();
      expect(savedConfigFile.getConfig('base').isChanged()).to.be.true();
    });

    it('should not write when the mutator throws', () => {
      const before = seedConfigFile();

      const repository = new ConfigFileJsonRepository(identityMigration, homeDir, createDefaults);

      expect(() => repository.update(() => {
        throw new Error('mutator failed');
      })).to.throw('mutator failed');

      expect(fs.readFileSync(configFilePath, 'utf8')).to.equal(before);
      expect(fs.existsSync(homeDir.joinPath('.config.json.lock'))).to.be.false();
    });

    // A command that reconfigures a node holds the lock for its whole run and
    // still saves through the ordinary paths. Taking the lock again for each of
    // those saves would leave it waiting on itself until the acquire timeout.
    it('should let a held lock be used by update() and write() without deadlocking', function it1() {
      this.timeout(10000);

      seedConfigFile();

      const repository = new ConfigFileJsonRepository(identityMigration, homeDir, createDefaults);

      repository.acquire();

      try {
        repository.update((configFile) => {
          configFile.getConfig('base').set('description', 'via-update');
        });

        const configFile = repository.read();
        configFile.getConfig('base').set('description', 'via-write');
        repository.write(configFile);
      } finally {
        repository.release();
      }

      const reread = new ConfigFileJsonRepository(identityMigration, homeDir, createDefaults).read();

      expect(reread.getConfig('base').get('description')).to.equal('via-write');
    });

    it('should let onSaved re-enter a repository lock taken by update', () => {
      seedConfigFile();

      const repository = new ConfigFileJsonRepository(
        identityMigration,
        homeDir,
        createDefaults,
        { acquireTimeout: 50 },
      );

      expect(() => repository.update((configFile) => {
        configFile.getConfig('base').set('description', 'outer-save');
      }, {
        onSaved: (configFile) => {
          configFile.getConfig('base').set('description', 'nested-save');
          repository.write(configFile);
        },
      })).to.not.throw();

      expect(repository.read().getConfig('base').get('description')).to.equal('nested-save');
    });

    it('should preserve an explicit lease acquired inside a locked callback', () => {
      seedConfigFile();

      const repository = new ConfigFileJsonRepository(identityMigration, homeDir, createDefaults);
      const lockPath = homeDir.joinPath('.config.json.lock');

      repository.update(() => {}, {
        onSaved: () => repository.acquire(),
      });

      expect(fs.existsSync(lockPath)).to.be.true();

      repository.release();

      expect(fs.existsSync(lockPath)).to.be.false();
    });

    it('should hold the lock between acquire and release, and free it after', () => {
      seedConfigFile();

      const repository = new ConfigFileJsonRepository(identityMigration, homeDir, createDefaults);
      const lockPath = homeDir.joinPath('.config.json.lock');

      repository.acquire();

      expect(fs.existsSync(lockPath)).to.be.true();

      repository.release();

      expect(fs.existsSync(lockPath)).to.be.false();
    });

    it('should keep an outer lease held until every acquire is released', () => {
      seedConfigFile();

      const repository = new ConfigFileJsonRepository(identityMigration, homeDir, createDefaults);
      const lockPath = homeDir.joinPath('.config.json.lock');

      repository.acquire();
      repository.acquire();
      repository.release();

      expect(fs.existsSync(lockPath)).to.be.true();

      repository.release();

      expect(fs.existsSync(lockPath)).to.be.false();
    });

    // Release runs from several exit paths - normal finish, command failure and
    // graceful shutdown - so calling it when nothing is held has to be harmless.
    it('should tolerate release without acquire, and repeated release', () => {
      seedConfigFile();

      const repository = new ConfigFileJsonRepository(identityMigration, homeDir, createDefaults);

      expect(() => repository.release()).to.not.throw();

      repository.acquire();
      repository.release();

      expect(() => repository.release()).to.not.throw();
    });

    // Exclusivity is the whole guarantee for a command that holds the lock across
    // its run, so losing it has to stop the save rather than pass unnoticed.
    // Reproduced by removing the lock directory out from under a live holder and
    // letting its refresh discover that, which is what a stolen lock looks like.
    it('should refuse to save after losing a lock it believed it held', async function it2() {
      this.timeout(10000);

      seedConfigFile();

      // proper-lockfile floors `stale` at 2s and the refresh that detects loss at
      // 1s, so this is as fast as the path can be made to happen.
      const repository = new ConfigFileJsonRepository(identityMigration, homeDir, createDefaults, CURRENT_FORMAT_VERSION, {
        stale: 2000,
      });

      repository.acquire();

      try {
        fs.rmdirSync(homeDir.joinPath('.config.json.lock'));

        await new Promise((resolve) => { setTimeout(resolve, 1500); });

        let renders = 0;
        let error;
        try {
          repository.update((configFile) => {
            configFile.getConfig('base').set('description', 'must-not-be-saved');
          }, {
            onSaved: () => {
              renders += 1;
            },
          });
        } catch (e) {
          error = e;
        }

        expect(error).to.be.instanceOf(Error);
        expect(error.message).to.include('Lost the lock');
        expect(error.message).to.include('.config.json.rescue-');
        expect(renders, 'a lost owner must not render stale service files').to.equal(0);

        repository.migrateConfigFile = (data) => ({
          ...data,
          configFormatVersion: '9.9.9',
        });
        repository.configFormatVersion = '9.9.9';

        let migrationRenders = 0;
        expect(() => repository.readAndMigrate({}, () => {
          migrationRenders += 1;
        })).to.throw('Lost the configuration lock');
        expect(migrationRenders, 'a lost owner must not render a migration').to.equal(0);
      } finally {
        repository.release();
      }

      const reread = new ConfigFileJsonRepository(identityMigration, homeDir, createDefaults).read();

      expect(reread.getConfig('base').get('description')).to.not.equal('must-not-be-saved');

      const [rescueName] = fs.readdirSync(homeDir.getPath())
        .filter((n) => n.startsWith('.config.json.rescue-'));
      const rescuePath = homeDir.joinPath(rescueName);
      expect(JSON.parse(fs.readFileSync(rescuePath, 'utf8'))
        .configs.base.description).to.equal('must-not-be-saved');
      // eslint-disable-next-line no-bitwise
      expect(fs.statSync(rescuePath).mode & 0o777).to.equal(0o600);

      repository.update((configFile) => {
        configFile.getConfig('base').set('description', 'saved-after-new-lock');
      });

      expect(repository.read().getConfig('base').get('description'))
        .to.equal('saved-after-new-lock');
    });

    // A waiter should be told what is happening in dashmate's terms, and only
    // after actually waiting - not handed the locking library's wording.
    it('should give up waiting for another holder with an actionable error', () => {
      seedConfigFile();

      const repository = new ConfigFileJsonRepository(identityMigration, homeDir, createDefaults, CURRENT_FORMAT_VERSION, {
        acquireTimeout: 700,
      });

      // Someone else holds it, exactly as proper-lockfile represents it
      fs.mkdirSync(homeDir.joinPath('.config.json.lock'));

      const startedAt = Date.now();

      let error;
      try {
        repository.update((configFile) => {
          configFile.getConfig('base').set('description', 'never-applied');
        });
      } catch (e) {
        error = e;
      }

      expect(error).to.be.instanceOf(Error);
      expect(error.message).to.include(homeDir.joinPath('.config.json.lock'));
      expect(error.message).to.include('dashmate helper');
      expect(error.message).to.include('reindex');
      expect(error.message).to.include('about a minute');

      expect(Date.now() - startedAt).to.be.at.least(600);

      fs.rmdirSync(homeDir.joinPath('.config.json.lock'));
    });

    // Reading fresh inside the lock is only sound if the lock is actually
    // honoured across processes. proper-lockfile represents a held lock as a
    // directory, so another process needs nothing but `fs` to hold it.
    it('should wait for a lock held by another process', function it0(done) {
      this.timeout(20000);

      seedConfigFile();

      const repository = new ConfigFileJsonRepository(identityMigration, homeDir, createDefaults);
      const lockPath = homeDir.joinPath('.config.json.lock');
      const holdMs = 700;
      const childScript = `
        const fs = require('fs');
        const lockPath = ${JSON.stringify(lockPath)};
        const configFilePath = ${JSON.stringify(configFilePath)};
        fs.mkdirSync(lockPath);
        setTimeout(() => {
          const data = JSON.parse(fs.readFileSync(configFilePath, 'utf8'));
          data.configs.base.description = 'written-by-lock-holder';
          fs.writeFileSync(configFilePath, JSON.stringify(data, undefined, 2) + '\\n');
          fs.rmdirSync(lockPath);
        }, ${holdMs});
      `;

      lockHolder = spawn(
        process.execPath,
        ['-e', childScript],
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
        configFile.getConfig('base').set('core.rpc.port', 30000);
      });

      expect(Date.now() - startedAt).to.be.at.least(holdMs / 2);

      const reread = new ConfigFileJsonRepository(identityMigration, homeDir, createDefaults).read();

      expect(reread.getConfig('base').get('description')).to.equal('written-by-lock-holder');
      expect(reread.getConfig('base').get('core.rpc.port')).to.equal(30000);

      lockHolder.on('exit', () => done());
    });
  });
});
