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

    // Service files and the config file are two writes. A process killed
    // between them leaves the generated files describing a value the config
    // file never got, and nothing in either file says so.
    describe('interrupted between rendering and saving', () => {
      it('should record that a render is owed while it is in flight', () => {
        seedConfigFile();

        const repository = new ConfigFileJsonRepository(
          identityMigration,
          homeDir,
          createDefaults,
          CURRENT_FORMAT_VERSION,
        );

        let markedWhileRendering;

        repository.update((configFile) => {
          configFile.getConfig('base').set('description', 'rendered');
        }, {
          beforeSave: () => {
            markedWhileRendering = repository.isRenderPending();
          },
        });

        expect(markedWhileRendering, 'marker should exist while rendering').to.be.true();
        expect(repository.isRenderPending(), 'marker should be gone once saved').to.be.false();
      });

      it('should clear only the render debt owned by the caller', () => {
        const repository = new ConfigFileJsonRepository(
          identityMigration,
          homeDir,
          createDefaults,
          CURRENT_FORMAT_VERSION,
        );

        const firstDebt = repository.markRenderPending();
        const secondDebt = repository.markRenderPending();

        expect(firstDebt).to.not.equal(secondDebt);

        repository.clearRenderPending(firstDebt);

        expect(repository.isRenderPending()).to.be.true();

        repository.clearRenderPending(secondDebt);

        expect(repository.isRenderPending()).to.be.false();
      });

      it('should re-render from the config file that survived', () => {
        const seeded = seedConfigFile();

        const repository = new ConfigFileJsonRepository(
          identityMigration,
          homeDir,
          createDefaults,
          CURRENT_FORMAT_VERSION,
        );

        // Exactly the state a kill between the render and the save leaves: the
        // marker present, the config file untouched.
        repository.markRenderPending();

        const rendered = [];

        expect(repository.recoverPendingRender((config) => rendered.push(config.getName())))
          .to.be.true();

        expect(rendered).to.deep.equal(['base']);
        expect(fs.readFileSync(configFilePath, 'utf8')).to.equal(seeded);
        expect(repository.isRenderPending()).to.be.false();
      });

      it('should do nothing and take no lock when no render is owed', () => {
        seedConfigFile();

        const repository = new ConfigFileJsonRepository(
          identityMigration,
          homeDir,
          createDefaults,
          CURRENT_FORMAT_VERSION,
        );

        let lockedDuringRecovery = false;

        expect(repository.recoverPendingRender(() => {
          lockedDuringRecovery = true;
        })).to.be.false();

        expect(lockedDuringRecovery).to.be.false();
        expect(fs.existsSync(homeDir.joinPath('.config.json.lock'))).to.be.false();
      });

      it('should keep the marker when the save fails and re-rendering also fails', () => {
        seedConfigFile();

        const repository = new ConfigFileJsonRepository(
          identityMigration,
          homeDir,
          createDefaults,
          CURRENT_FORMAT_VERSION,
        );

        let renders = 0;

        expect(() => repository.update((configFile) => {
          configFile.getConfig('base').set('description', 'never-saved');
        }, {
          beforeSave: () => {
            renders += 1;

            if (renders === 1) {
              // Simulate the atomic replace failing after the render.
              fs.chmodSync(homeDir.getPath(), 0o500);
            } else {
              throw new Error('render failed too');
            }
          },
        })).to.throw();

        fs.chmodSync(homeDir.getPath(), 0o755);

        expect(repository.isRenderPending(), 'marker must outlive a failed recovery').to.be.true();
      });

      it('should retry recovery when rendering fails', () => {
        seedConfigFile();

        const repository = new ConfigFileJsonRepository(
          identityMigration,
          homeDir,
          createDefaults,
          CURRENT_FORMAT_VERSION,
        );
        repository.markRenderPending();

        expect(() => repository.recoverPendingRender(() => {
          throw new Error('render failed');
        })).to.throw('render failed');
        expect(repository.isRenderPending()).to.be.true();

        expect(repository.recoverPendingRender(() => {})).to.be.true();
        expect(repository.isRenderPending()).to.be.false();
      });
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

    // Effects registered by the caller run before the lock is released, so a
    // concurrent command cannot slip between the save and them.
    // Service files are what a change actually does; committing the JSON
    // without them leaves the node running the old value with nothing to make
    // it try again, because a config read back off disk is clean.
    it('should commit nothing when rendering service files fails', () => {
      seedConfigFile();

      const repository = new ConfigFileJsonRepository(identityMigration, homeDir, createDefaults);
      const before = fs.readFileSync(configFilePath, 'utf8');

      expect(() => repository.update((configFile) => {
        configFile.getConfig('base').set('description', 'never-committed');
      }, {
        beforeSave: () => {
          throw new Error('template write failed');
        },
      })).to.throw('template write failed');

      expect(fs.readFileSync(configFilePath, 'utf8')).to.equal(before);
      expect(repository.read().getConfig('base').get('description')).to.not.equal('never-committed');
    });

    it('should render service files before the change reaches disk', () => {
      seedConfigFile();

      const repository = new ConfigFileJsonRepository(identityMigration, homeDir, createDefaults);

      let onDiskWhenRendering;

      repository.update((configFile) => {
        configFile.getConfig('base').set('description', 'rendered-first');
      }, {
        beforeSave: () => {
          onDiskWhenRendering = JSON.parse(fs.readFileSync(configFilePath, 'utf8'))
            .configs.base.description;
        },
      });

      expect(onDiskWhenRendering).to.not.equal('rendered-first');
      expect(repository.read().getConfig('base').get('description')).to.equal('rendered-first');
    });

    it('should keep repository locks outside the config-name namespace', () => {
      seedConfigFile();

      const repository = new ConfigFileJsonRepository(identityMigration, homeDir, createDefaults);

      repository.update((configFile) => {
        configFile.getConfig('base').set('description', 'changed');
      }, {
        beforeSave: () => {
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
            beforeSave: () => {
              renders += 1;
            },
          });
        } catch (e) {
          error = e;
        }

        expect(error).to.be.instanceOf(Error);
        expect(error.message).to.include('Lost the lock');
        expect(error.message).to.include('.config.json.rescue');
        expect(renders, 'a lost owner must not render stale service files').to.equal(0);
        expect(repository.isRenderPending(), 'no render means no recovery debt').to.be.false();

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

      const rescuePath = homeDir.joinPath('.config.json.rescue');
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
