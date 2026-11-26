import fs from 'fs';
import { spawn } from 'child_process';
import { expect } from 'chai';
import lockfile from 'proper-lockfile';
import tenderdashSeeds from '../../../../configs/defaults/tenderdashSeeds.js';
import seedSetHash from '../../../../src/tenderdash/seedSetHash.js';
import renderServiceTemplatesFactory from '../../../../src/templates/renderServiceTemplatesFactory.js';
import renderTemplateFactory from '../../../../src/templates/renderTemplateFactory.js';
import HomeDir from '../../../../src/config/HomeDir.js';
import getBaseConfigFactory from '../../../../configs/defaults/getBaseConfigFactory.js';
import ConfigFile from '../../../../src/config/configFile/ConfigFile.js';
import ConfigFileJsonRepository from '../../../../src/config/configFile/ConfigFileJsonRepository.js';
import getConfigFormatVersion from '../../../../src/config/configFile/getConfigFormatVersion.js';
import migrateConfigFileFactory from '../../../../src/config/configFile/migrateConfigFileFactory.js';

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

  ['mainnet', 'testnet'].forEach(network => {
    it(`should refresh ${network} stock seeds under lock without a format bump and retry a failed render`, () => {
      const originalDefaults = structuredClone(tenderdashSeeds[network]);
      try {
        const data = JSON.parse(seedConfigFile());
        data.configs.base.network = network;
        const { tenderdash } = data.configs.base.platform.drive;
        const self = { id: 'e'.repeat(40), host: '192.0.2.1', port: 26656 };
        const peer = { id: 'f'.repeat(40), host: '192.0.2.2', port: 26656 };
        tenderdash.node.id = self.id;
        tenderdash.p2p.seeds = originalDefaults.seeds;
        const original = JSON.stringify(data);
        fs.writeFileSync(configFilePath, original);
        tenderdashSeeds[network].previousSeedSetHashes.push(seedSetHash(originalDefaults.seeds));
        tenderdashSeeds[network].seeds = [self, peer];
        const repository = new ConfigFileJsonRepository(
          identityMigration,
          homeDir,
          createDefaults,
          CURRENT_FORMAT_VERSION,
        );
        expect(() => repository.readAndMigrate({}, ([config]) => {
          expect(fs.existsSync(homeDir.joinPath('.config.json.lock'))).to.equal(true);
          expect(config.get('platform.drive.tenderdash.p2p.seeds')).to.deep.equal([peer]);
          throw new Error('render failed');
        })).to.throw('render failed');
        expect(fs.readFileSync(configFilePath, 'utf8')).to.equal(original);
        repository.readAndMigrate({}, configs => expect(configs).to.have.length(1));
        const saved = JSON.parse(fs.readFileSync(configFilePath, 'utf8'));
        expect(saved.configFormatVersion).to.equal(CURRENT_FORMAT_VERSION);
        expect(saved.configs.base.platform.drive.tenderdash.p2p.seeds).to.deep.equal([peer]);
        repository.readAndMigrate({}, () => { throw new Error('must not render twice'); });
      } finally {
        tenderdashSeeds[network] = originalDefaults;
      }
    });
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

    it('should honor recovery validation bypasses when refreshing seeds and completing a fullnode identity', () => {
      const defaults = tenderdashSeeds.mainnet;
      const oldSeeds = defaults.seeds.slice(0, 1);
      const history = defaults.previousSeedSetHashes;
      try {
        defaults.previousSeedSetHashes = [...history, seedSetHash(oldSeeds)];
        const data = JSON.parse(seedConfigFile());
        data.configs.node1 = structuredClone(data.configs.base);
        Object.assign(data.configs.node1, { network: 'mainnet', description: { invalid: true } });
        data.configs.node1.core.masternode.enable = false;
        data.configs.node1.platform.drive.tenderdash.p2p.seeds = oldSeeds;
        const original = JSON.stringify(data);
        fs.writeFileSync(configFilePath, original);
        const repository = new ConfigFileJsonRepository(
          identityMigration,
          homeDir,
          createDefaults,
          CURRENT_FORMAT_VERSION,
        );
        const render = renderServiceTemplatesFactory(renderTemplateFactory());
        expect(() => repository.read()).to.throw('description');

        for (const skipValidation of [true, ({ name }) => name === 'node1']) {
          fs.writeFileSync(configFilePath, original);
          let renderedKey;
          const { configFile } = repository.readAndMigrate({ skipValidation }, ([config]) => {
            renderedKey = JSON.parse(render(config)['platform/drive/tenderdash/node_key.json']).priv_key.value;
          });
          const config = configFile.getConfig('node1');
          expect(config.get('description')).to.deep.equal({ invalid: true });
          expect(config.get('platform.drive.tenderdash.p2p.seeds')).to.deep.equal(defaults.seeds);
          expect(renderedKey).to.be.a('string').and.not.to.equal('null');
          const saved = JSON.parse(fs.readFileSync(configFilePath, 'utf8'));
          expect(saved.configs.node1.platform.drive.tenderdash.node.key).to.equal(renderedKey);
        }
      } finally {
        defaults.previousSeedSetHashes = history;
      }
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

    it('should store a changed fullnode without generating a node identity', () => {
      // A node's private key comes from the command that needs it. Saving
      // must not invent one - every incidental save, and the rescue copy
      // written after losing the lock, would otherwise carry a fresh key.
      const repository = new ConfigFileJsonRepository(
        identityMigration,
        homeDir,
        createDefaults,
        CURRENT_FORMAT_VERSION,
      );
      const configFile = createDefaults();
      configFile.createConfig('node1', 'base');
      configFile.getConfig('node1').set('core.masternode.enable', false);

      repository.write(configFile);

      const saved = JSON.parse(fs.readFileSync(configFilePath, 'utf8'));
      expect(saved.configs.node1.platform.drive.tenderdash.node)
        .to.deep.equal({ id: null, key: null });
      expect(configFile.getConfig('node1').get('platform.drive.tenderdash.node.key')).to.equal(null);
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

    it('should retry every migrated identity when rendering fails before the save', () => {
      const configFile = createDefaults();
      for (const name of ['node1', 'node2']) {
        configFile.createConfig(name, 'base');
        configFile.getConfig(name).set('core.masternode.enable', false);
      }
      const original = JSON.stringify(configFile.toObject());
      fs.writeFileSync(configFilePath, original);
      const repository = new ConfigFileJsonRepository(
        (data) => ({ ...data, configFormatVersion: '9.9.9' }),
        homeDir,
        createDefaults,
        '9.9.9',
      );
      const render = renderServiceTemplatesFactory(renderTemplateFactory());

      expect(() => repository.readAndMigrate({}, (configs) => {
        const firstNode = configs.find(config => config.getName() === 'node1');
        render(firstNode);
        expect(firstNode.get('platform.drive.tenderdash.node.key')).to.be.a('string');
        throw new Error('template write failed');
      })).to.throw('template write failed');
      expect(fs.readFileSync(configFilePath, 'utf8')).to.equal(original);

      const rendered = {};
      repository.readAndMigrate({}, (configs) => {
        configs.forEach(config => {
          rendered[config.getName()] = JSON.parse(
            render(config)['platform/drive/tenderdash/node_key.json'],
          );
        });
      });
      const saved = JSON.parse(fs.readFileSync(configFilePath, 'utf8'));
      expect(saved.configFormatVersion).to.equal('9.9.9');
      for (const name of ['node1', 'node2']) {
        expect(saved.configs[name].platform.drive.tenderdash.node.key)
          .to.equal(rendered[name].priv_key.value).and.to.be.a('string');
      }
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

  // The dashmate helper container keeps running the version that started it
  // after the CLI is upgraded by hand, and an older binary can stay on PATH.
  // Migrating only runs forward, so such a build reads a file a newer build
  // already migrated as if it were current. Saving from there would write its
  // own state under the newer stamp, and the newer build would never migrate
  // it again.
  describe('a config file updated by a newer dashmate', () => {
    const NEWER_IMAGE = 'dashpay/envoy:newer-migration';
    const NEWER_FORMAT_ERROR = /updated by a newer dashmate/;

    let newerFormatVersion;
    let newer;
    let older;

    beforeEach(() => {
      const migrations = {
        '4.2.0': (data) => {
          Object.values(data.configs).forEach((options) => {
            // eslint-disable-next-line no-param-reassign
            options.platform.gateway.docker.image = NEWER_IMAGE;
          });

          return data;
        },
      };
      const { version } = JSON.parse(fs.readFileSync(new URL('../../../../package.json', import.meta.url), 'utf8'));

      newerFormatVersion = getConfigFormatVersion(migrations, version);

      newer = new ConfigFileJsonRepository(
        migrateConfigFileFactory(() => migrations),
        homeDir,
        createDefaults,
        newerFormatVersion,
      );
      older = new ConfigFileJsonRepository(
        identityMigration,
        homeDir,
        createDefaults,
        CURRENT_FORMAT_VERSION,
        { acquireTimeout: 50 },
      );

      seedConfigFile();
      newer.readAndMigrate({}, () => {});
    });

    it('should refuse to save an older build\'s state over the newer migration', () => {
      const migrated = fs.readFileSync(configFilePath, 'utf8');

      expect(JSON.parse(migrated).configFormatVersion).to.equal(newerFormatVersion);

      // What an older `dashmate update` or helper renewal does: read, set a
      // value it owns with its own meaning, save.
      const configFile = older.read();
      configFile.getConfig('base').set('platform.gateway.docker.image', 'dashpay/envoy:older');

      expect(() => older.write(configFile)).to.throw(NEWER_FORMAT_ERROR);

      expect(fs.readFileSync(configFilePath, 'utf8')).to.equal(migrated);
      expect(newer.read().getConfig('base').get('platform.gateway.docker.image')).to.equal(NEWER_IMAGE);
    });

    // Renewal reports a refusal from acquire() as one that asked the authority
    // for nothing. A refusal at a later save can follow a certificate request,
    // so it must not be marked the same way.
    it('should mark only the refusal made when the lock is taken as before any work', () => {
      const refusalOf = (fn) => {
        try {
          fn();
        } catch (e) {
          return e;
        }

        return null;
      };

      expect(refusalOf(() => older.acquire())).to.have.property('isBeforeAnyWork', true);
      expect(refusalOf(() => older.write(older.read()))).to.have.property('isBeforeAnyWork', false);
      expect(refusalOf(() => older.update(() => {}))).to.have.property('isBeforeAnyWork', false);
    });

    // Diagnostic only. A restart reloads the gateway from the certificate files
    // on disk, which nothing here has checked, so the refusal must not advise one.
    it('should name both versions and the newer dashmate in the refusal, without advising a restart', () => {
      expect(() => older.write(older.read())).to.throw(Error)
        .with.property('message')
        .that.includes(newerFormatVersion)
        .and.includes(CURRENT_FORMAT_VERSION)
        .and.includes('newer dashmate')
        .and.not.includes('dashmate restart');
    });

    it('should refuse update() without running onSaved and release the lock', function it() {
      const migrated = fs.readFileSync(configFilePath, 'utf8');
      const onSaved = this.sinon.stub();

      expect(() => older.update((configFile) => {
        configFile.getConfig('base').set('description', 'older edit');
      }, { onSaved })).to.throw(NEWER_FORMAT_ERROR);

      expect(onSaved).to.not.have.been.called();
      expect(fs.readFileSync(configFilePath, 'utf8')).to.equal(migrated);
      expect(fs.existsSync(homeDir.joinPath('.config.json.lock'))).to.be.false();
    });

    it('should name the newer format, not a schema error, when update() meets a newer shape', () => {
      const data = JSON.parse(fs.readFileSync(configFilePath, 'utf8'));
      data.configs.base.platform.gateway.optionOnlyANewerBuildKnows = true;
      fs.writeFileSync(configFilePath, JSON.stringify(data));

      expect(() => older.read()).to.throw(/Invalid .* config format/);
      expect(() => older.update(() => {})).to.throw(NEWER_FORMAT_ERROR);
    });

    // A lock-holding command or a certificate renewal does its long work after
    // acquiring. Refusing only at the save would let a renewal spend a
    // certificate issuance first.
    it('should refuse acquire() before any work and leave the lock free', () => {
      expect(() => older.acquire()).to.throw(NEWER_FORMAT_ERROR);

      expect(fs.existsSync(homeDir.joinPath('.config.json.lock'))).to.be.false();

      // Nothing is left held: release() is a no-op and the newer build gets
      // the lock straight away.
      older.release();
      newer.acquire();
      newer.release();
    });

    it('should still read the file, so an older build can report', () => {
      expect(older.read().getConfig('base').get('platform.gateway.docker.image')).to.equal(NEWER_IMAGE);
      expect(older.readAndMigrate().configFile.getConfigFormatVersion()).to.equal(newerFormatVersion);
    });

    ['mainnet', 'testnet'].forEach((network) => {
      it(`should not refresh ${network} stock seeds or render from an older build`, function it() {
        const originalDefaults = structuredClone(tenderdashSeeds[network]);

        try {
          const data = JSON.parse(fs.readFileSync(configFilePath, 'utf8'));
          data.configs.base.network = network;
          data.configs.base.platform.drive.tenderdash.p2p.seeds = originalDefaults.seeds;
          const original = JSON.stringify(data);
          fs.writeFileSync(configFilePath, original);

          // The older build ships a seed table that wants to replace these.
          tenderdashSeeds[network].previousSeedSetHashes.push(seedSetHash(originalDefaults.seeds));
          tenderdashSeeds[network].seeds = [{ id: 'f'.repeat(40), host: '8.8.4.4', port: 26656 }];

          const onMigrated = this.sinon.stub();
          // Reading is all this build may do with the file, and a read takes
          // no lock - so it cannot hold up the newer build either.
          this.sinon.spy(lockfile, 'lockSync');

          const { configFile } = older.readAndMigrate({}, onMigrated);

          expect(lockfile.lockSync).to.not.have.been.called();
          expect(onMigrated).to.not.have.been.called();
          expect(fs.readFileSync(configFilePath, 'utf8')).to.equal(original);

          // What `config get --raw` reports has to be what the file holds. The
          // older build's replacement was never saved, so showing it would
          // describe a configuration this node does not have.
          const seedsPath = 'platform.drive.tenderdash.p2p.seeds';
          [configFile, older.read()].forEach((read) => {
            expect(read.getConfig('base').getStored(seedsPath)).to.deep.equal(originalDefaults.seeds);
            expect(read.isChanged()).to.be.false();
          });
        } finally {
          tenderdashSeeds[network] = originalDefaults;
        }
      });
    });

    // The probe that decides whether to take the lock reads without it. A newer
    // build can migrate the file while this one waits, and what it then reads
    // under the lock is what counts.
    it('should not render when a newer build migrates the file while waiting for the lock', function it() {
      const originalDefaults = structuredClone(tenderdashSeeds.testnet);

      try {
        const data = JSON.parse(fs.readFileSync(configFilePath, 'utf8'));
        data.configs.base.network = 'testnet';
        data.configs.base.platform.drive.tenderdash.p2p.seeds = originalDefaults.seeds;
        const migratedByNewer = JSON.stringify(data);
        fs.writeFileSync(configFilePath, JSON.stringify({
          ...data,
          configFormatVersion: CURRENT_FORMAT_VERSION,
        }));

        tenderdashSeeds.testnet.previousSeedSetHashes.push(seedSetHash(originalDefaults.seeds));
        tenderdashSeeds.testnet.seeds = [{ id: 'f'.repeat(40), host: '8.8.4.4', port: 26656 }];

        const { lockSync } = lockfile;
        this.sinon.stub(lockfile, 'lockSync').callsFake((...args) => {
          fs.writeFileSync(configFilePath, migratedByNewer);

          return lockSync(...args);
        });

        const onMigrated = this.sinon.stub();

        const { configFile } = older.readAndMigrate({}, onMigrated);

        expect(lockfile.lockSync).to.have.been.calledOnce();
        expect(onMigrated).to.not.have.been.called();
        expect(configFile.getConfigFormatVersion()).to.equal(newerFormatVersion);
        expect(fs.readFileSync(configFilePath, 'utf8')).to.equal(migratedByNewer);
        expect(fs.existsSync(homeDir.joinPath('.config.json.lock'))).to.be.false();
      } finally {
        tenderdashSeeds.testnet = originalDefaults;
      }
    });

    it('should let an older build create the file when there is none', () => {
      fs.rmSync(configFilePath);

      older.update(() => {});

      expect(JSON.parse(fs.readFileSync(configFilePath, 'utf8')).configFormatVersion)
        .to.equal(CURRENT_FORMAT_VERSION);
    });
  });

  // Skipping the seed refresh is for a file a newer build owns. Anything this
  // build cannot order as newer still gets the refresh it always had.
  [
    ['an older stamp', '4.0.0', CURRENT_FORMAT_VERSION],
    ['a stamp that is not semver', 'not-a-version', CURRENT_FORMAT_VERSION],
    ['no format version of its own', '9.0.0', undefined],
  ].forEach(([name, recorded, own]) => {
    it(`should still refresh obsolete stock seeds for ${name}`, () => {
      const originalDefaults = structuredClone(tenderdashSeeds.testnet);

      try {
        const data = JSON.parse(seedConfigFile());
        data.configFormatVersion = recorded;
        data.configs.base.network = 'testnet';
        data.configs.base.platform.drive.tenderdash.p2p.seeds = originalDefaults.seeds;
        fs.writeFileSync(configFilePath, JSON.stringify(data));

        tenderdashSeeds.testnet.previousSeedSetHashes.push(seedSetHash(originalDefaults.seeds));
        tenderdashSeeds.testnet.seeds = [{ id: 'f'.repeat(40), host: '8.8.4.4', port: 26656 }];

        const configFile = new ConfigFileJsonRepository(identityMigration, homeDir, createDefaults, own).read();

        expect(configFile.getConfig('base').getStored('platform.drive.tenderdash.p2p.seeds'))
          .to.deep.equal(tenderdashSeeds.testnet.seeds);
        expect(configFile.isChanged()).to.be.true();
      } finally {
        tenderdashSeeds.testnet = originalDefaults;
      }
    });
  });

  describe('format version order', () => {
    /**
     * @param {string} recorded - stamped on disk
     * @param {string} own - the format the writing build produces
     */
    function writeOver(recorded, own) {
      const data = JSON.parse(seedConfigFile());
      data.configFormatVersion = recorded;
      fs.writeFileSync(configFilePath, JSON.stringify(data));

      const repository = new ConfigFileJsonRepository(identityMigration, homeDir, createDefaults, own);

      return () => repository.write(repository.read());
    }

    [
      ['5.1.0-dev.3', '5.0.2'],
      ['5.1.0-dev.7', '5.1.0-dev.5'],
      ['5.1.0', '5.1.0-dev.9'],
    ].forEach(([recorded, own]) => {
      it(`should refuse a ${own} build writing over ${recorded}`, () => {
        expect(writeOver(recorded, own)).to.throw(/updated by a newer dashmate/);
      });
    });

    [
      ['5.1.0', '5.1.0'],
      ['5.1.0-dev.7', '5.1.0'],
      ['5.0.2', '5.1.0-dev.3'],
    ].forEach(([recorded, own]) => {
      it(`should let a ${own} build write over ${recorded}`, () => {
        expect(writeOver(recorded, own)).to.not.throw();
      });
    });
  });
});
