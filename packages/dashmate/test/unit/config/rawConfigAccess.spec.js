import { expect } from 'chai';
import fs from 'fs';
import path from 'path';
import { PACKAGE_ROOT_DIR } from '../../../src/constants.js';

/**
 * Options with a version-derived default are stored unset, so a raw read hands
 * back null where an image is expected. Ordinary reads resolve that; raw reads
 * exist for the few places that must see stored intent instead.
 *
 * The design only holds while that set stays small and deliberate, and the
 * failure mode is quiet - a new command reading raw state prints null instead of
 * the image the node will run. So the allowlist is asserted rather than trusted.
 */
const ALLOWED_RAW_READERS = {
  // persistence: writing a resolved value back would defeat the whole design
  'src/config/configFile/ConfigFile.js': 2,
  // equality compares what the operator chose, not what it resolves to
  'src/config/Config.js': null,
  // reset restores stored intent
  'src/listr/tasks/resetNodeTaskFactory.js': 1,
  // base config is inherited by each network config, intent included
  'configs/defaults/getTestnetConfigFactory.js': 1,
  'configs/defaults/getLocalConfigFactory.js': 1,
  'configs/defaults/getMainnetConfigFactory.js': 1,
  // the commands that deliberately expose stored state behind --raw
  'src/commands/config/index.js': 1,
  'src/commands/config/get.js': 1,
};

function collectJsFiles(directory, collected = []) {
  for (const entry of fs.readdirSync(directory, { withFileTypes: true })) {
    const entryPath = path.join(directory, entry.name);

    if (entry.isDirectory()) {
      collectJsFiles(entryPath, collected);
    } else if (entry.name.endsWith('.js')) {
      collected.push(entryPath);
    }
  }

  return collected;
}

describe('raw config access', () => {
  it('should be limited to the places that need stored intent', () => {
    const roots = ['src', 'configs'].map((dir) => path.join(PACKAGE_ROOT_DIR, dir));
    const files = roots.flatMap((root) => collectJsFiles(root));

    const offenders = [];

    for (const file of files) {
      const relativePath = path.relative(PACKAGE_ROOT_DIR, file);
      const matches = fs.readFileSync(file, 'utf8').match(/getStoredOptions\(|getStored\(/g);

      if (matches === null) {
        continue;
      }

      if (!(relativePath in ALLOWED_RAW_READERS)) {
        offenders.push(`${relativePath} reads stored config state but is not on the allowlist`);
        continue;
      }

      const expectedCount = ALLOWED_RAW_READERS[relativePath];

      // Config.js defines the accessors, so counting its uses is meaningless.
      if (expectedCount !== null && matches.length !== expectedCount) {
        offenders.push(
          `${relativePath} has ${matches.length} raw reads, expected ${expectedCount}`,
        );
      }
    }

    expect(offenders).to.deep.equal(
      [],
      'raw config reads changed; an ordinary read resolves derived defaults, so use one unless this really needs stored intent',
    );
  });
});
