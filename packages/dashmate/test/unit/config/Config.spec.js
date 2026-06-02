import { expect } from 'chai';
import Config from '../../../src/config/Config.js';

describe('Config', () => {
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
});
