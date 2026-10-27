import { expect } from 'chai';
import HomeDir from '../../../src/config/HomeDir.js';
import getBaseConfigFactory from '../../../configs/defaults/getBaseConfigFactory.js';
import { DERIVED_DEFAULTS } from '../../../src/config/derivedDefaults.js';
import Config from '../../../src/config/Config.js';

describe('Config', () => {
  describe('constructor', () => {
    it('rejects path-like config names even when option validation is skipped', () => {
      for (const name of ['..', '../mainnet', 'slot/../mainnet', 'a/b', 'a\\b']) {
        expect(() => new Config(name, {}, true), name).to.throw('path-safe segment');
      }
    });
  });

  describe('change tracking', () => {
    let storedOptions;

    beforeEach(() => {
      storedOptions = getBaseConfigFactory(HomeDir.createTemp())().getStoredOptions();
    });

    // Loading a config off disk is not an edit to it. When hydration marks a
    // config dirty, every command - including pure readers like `config get` -
    // persists the whole config file on exit, and a slow reader can write its
    // stale snapshot over a concurrent `config set`.
    it('should not consider a freshly hydrated config changed', () => {
      const config = new Config('testnet', storedOptions);

      expect(config.isChanged()).to.be.false();
    });

    it('should consider a config changed after set()', () => {
      const config = new Config('testnet', storedOptions);

      config.set('description', 'changed');

      expect(config.isChanged()).to.be.true();
    });

    // setOptions() is a genuine mutation when called on an existing config
    // (resetNodeTaskFactory restores defaults through it), so only the
    // constructor's initial hydration is exempt.
    it('should consider a config changed after setOptions() post-construction', () => {
      const config = new Config('testnet', storedOptions);

      config.setOptions(storedOptions);

      expect(config.isChanged()).to.be.true();
    });
  });

  describe('.isSchemaPathAllowed', () => {
    // The bug that triggered this method: `dashmate config set
    // platform.drive.abci.docker.build.buildArgs.SDK_TEST_DATA true`
    // was failing because the old `config.get(path)` pre-check rejected
    // any path whose value isn't already stored — including new keys
    // inside map-shaped properties whose schema legally accepts them
    // via `additionalProperties: <schema>`. Each test pins one slice
    // of the schema-walk so a regression surfaces fast.

    describe('typed properties', () => {
      it('accepts a deeply-nested path that traverses only `properties`', () => {
        expect(
          Config.isSchemaPathAllowed('platform.drive.abci.docker.build.enabled'),
        ).to.be.true();
      });

      it('accepts a top-level property', () => {
        expect(Config.isSchemaPathAllowed('network')).to.be.true();
      });

      it('rejects a top-level typo', () => {
        // `platfom` (typo) is not in top-level `properties` and the schema
        // has `additionalProperties: false`, so it must not be allowed.
        expect(
          Config.isSchemaPathAllowed('platfom.drive.abci.docker.build.enabled'),
        ).to.be.false();
      });

      it('rejects a typo in the middle of the path', () => {
        expect(
          Config.isSchemaPathAllowed('platform.drive.abc.docker.build.enabled'),
        ).to.be.false();
      });
    });

    describe('map-shaped properties (additionalProperties: <schema>)', () => {
      it('accepts a new key inside a value-keyed map', () => {
        // The original failure. `buildArgs` is defined as
        // `{ type: 'object', additionalProperties: { type: 'string' } }`
        // so any key with a string value is schema-legal even if not yet set.
        expect(
          Config.isSchemaPathAllowed(
            'platform.drive.abci.docker.build.buildArgs.SDK_TEST_DATA',
          ),
        ).to.be.true();
      });

      it('accepts an arbitrary key inside the map (the whole point)', () => {
        expect(
          Config.isSchemaPathAllowed(
            'platform.drive.abci.docker.build.buildArgs.ANY_KEY_AT_ALL',
          ),
        ).to.be.true();
      });

      it('accepts the map property itself', () => {
        expect(
          Config.isSchemaPathAllowed('platform.drive.abci.docker.build.buildArgs'),
        ).to.be.true();
      });
    });

    describe('$ref traversal', () => {
      it('descends through a $ref to `#/definitions/dockerBuild`', () => {
        // `platform.drive.abci.docker.build` resolves via $ref to the shared
        // `dockerBuild` definition — buildArgs is defined there.
        expect(
          Config.isSchemaPathAllowed(
            'platform.drive.abci.docker.build.buildArgs.X',
          ),
        ).to.be.true();
      });

      it('descends through a $ref for a sibling Rust build (rs-dapi)', () => {
        expect(
          Config.isSchemaPathAllowed(
            'platform.dapi.rsDapi.docker.build.buildArgs.X',
          ),
        ).to.be.true();
      });
    });

    describe('edge cases', () => {
      it('rejects an empty path', () => {
        expect(Config.isSchemaPathAllowed('')).to.be.false();
      });

      it('rejects a non-string path', () => {
        expect(Config.isSchemaPathAllowed(null)).to.be.false();
        expect(Config.isSchemaPathAllowed(undefined)).to.be.false();
        expect(Config.isSchemaPathAllowed(42)).to.be.false();
      });

      it('rejects descending past a leaf primitive', () => {
        // `network` is a string at top level; you cannot index further.
        expect(Config.isSchemaPathAllowed('network.something')).to.be.false();
      });

      it('rejects paths with empty segments', () => {
        // Leading/trailing/double dots must not slip an empty key through a
        // map's `additionalProperties` descent.
        const buildArgs = 'platform.drive.abci.docker.build.buildArgs';
        expect(Config.isSchemaPathAllowed(`${buildArgs}.`)).to.be.false();
        expect(Config.isSchemaPathAllowed(`${buildArgs}..SDK_TEST_DATA`)).to.be.false();
        expect(Config.isSchemaPathAllowed('.network')).to.be.false();
      });
    });
  });

  describe('regression: paths the buggy pre-check rejected are now permitted', () => {
    // Before the fix, `dashmate config set` did a value-existence pre-check
    // (`config.get(path)`) that threw `InvalidOptionPathError` for any path
    // whose value wasn't already stored. That blocked legal sets under
    // map-shaped properties (`additionalProperties: <schema>`). The fix
    // replaced the pre-check with `isSchemaPathAllowed`. These tests pin
    // each path the original failure surfaced through.

    it('permits the original failing path (`…buildArgs.SDK_TEST_DATA`)', () => {
      expect(
        Config.isSchemaPathAllowed(
          'platform.drive.abci.docker.build.buildArgs.SDK_TEST_DATA',
        ),
      ).to.be.true();
    });

    it('permits the same shape for rs-dapi (parallel Rust build)', () => {
      expect(
        Config.isSchemaPathAllowed(
          'platform.dapi.rsDapi.docker.build.buildArgs.CARGO_BUILD_PROFILE',
        ),
      ).to.be.true();
    });

    it('still permits the canonical typed paths that the pre-check used to handle', () => {
      // Sanity: paths whose values DO exist after `dashmate setup local` —
      // the original pre-check used to gate these via `config.get`. The
      // schema walker must accept them too, or the CLI breaks for everyone.
      for (const path of [
        'platform.drive.abci.docker.build.enabled',
        'platform.drive.abci.docker.image',
        'platform.dapi.rsDapi.docker.build.enabled',
        'core.insight.enabled',
      ]) {
        expect(Config.isSchemaPathAllowed(path), `path: ${path}`).to.be.true();
      }
    });
  });

  describe('version-derived defaults', () => {
    // An unset derived option means "use the image line this dashmate build
    // ships". Ordinary reads have to fill that in, or an operator is shown null
    // where a real image will run - and the config command, doctor archives and
    // compose templates all read through different accessors.
    const DRIVE_IMAGE = 'platform.drive.abci.docker.image';

    let config;

    beforeEach(() => {
      config = getBaseConfigFactory(HomeDir.createTemp())();
    });

    it('should store nothing for a derived option', () => {
      expect(config.getStored(DRIVE_IMAGE)).to.equal(null);
      expect(config.getStoredOptions().platform.drive.abci.docker.image).to.equal(null);
    });

    it('should resolve a derived option on every kind of read', () => {
      const expected = DERIVED_DEFAULTS[DRIVE_IMAGE]();

      // exact path
      expect(config.get(DRIVE_IMAGE)).to.equal(expected);
      // parent object - the read that made the first attempt at this incoherent
      expect(config.get('platform.drive.abci.docker').image).to.equal(expected);
      // whole config
      expect(config.getOptions().platform.drive.abci.docker.image).to.equal(expected);
      // the property templates and reindex render through
      expect(config.options.platform.drive.abci.docker.image).to.equal(expected);
      // serialization
      // serialization - the shape doctor archives rebuild a config from
      expect(JSON.parse(JSON.stringify(config)).options.platform.drive.abci.docker.image)
        .to.equal(expected);
    });

    it('should keep an operator image on every read and store it', () => {
      config.set(DRIVE_IMAGE, 'registry.example.com/drive:patched');

      expect(config.get(DRIVE_IMAGE)).to.equal('registry.example.com/drive:patched');
      expect(config.get('platform.drive.abci.docker').image).to.equal('registry.example.com/drive:patched');
      expect(config.getStored(DRIVE_IMAGE)).to.equal('registry.example.com/drive:patched');
    });

    it('should return to tracking when the option is unset again', () => {
      config.set(DRIVE_IMAGE, 'registry.example.com/drive:patched');
      config.set(DRIVE_IMAGE, null);

      expect(config.getStored(DRIVE_IMAGE)).to.equal(null);
      expect(config.get(DRIVE_IMAGE)).to.equal(DERIVED_DEFAULTS[DRIVE_IMAGE]());
    });

    it('should compare configs by what they store, not what they resolve to', () => {
      const tracking = getBaseConfigFactory(HomeDir.createTemp())();
      const pinned = getBaseConfigFactory(HomeDir.createTemp())();

      pinned.set(DRIVE_IMAGE, DERIVED_DEFAULTS[DRIVE_IMAGE]());

      // identical effective images, different intent
      expect(pinned.get(DRIVE_IMAGE)).to.equal(tracking.get(DRIVE_IMAGE));
      expect(tracking.isEqual(pinned)).to.equal(false);
    });

    it('should refuse writes through a whole-config read', () => {
      // Persisting a resolved value would defeat the design, so the snapshot
      // these accessors hand out is frozen rather than quietly copied.
      expect(() => {
        config.getOptions().platform.drive.abci.docker.image = 'dashpay/drive:sneaky';
      }).to.throw();
    });

    it('should only allow null where a default exists to fill it', () => {
      // Widening another image to null would leave it with nothing to resolve to.
      expect(() => config.set('core.docker.image', null)).to.throw();

      const nullableByDefault = Object.keys(DERIVED_DEFAULTS);
      nullableByDefault.forEach((optionPath) => {
        expect(() => config.set(optionPath, null), optionPath).to.not.throw();
      });
    });
  });
});
