import { expect } from 'chai';
import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';
import dot from 'dot';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const TEMPLATE_PATH = path.resolve(
  __dirname,
  '../../../templates/dynamic-compose.yml.dot',
);

function render(buildArgsByService) {
  dot.templateSettings.strip = false;
  const tpl = fs.readFileSync(TEMPLATE_PATH, 'utf8');
  const fn = dot.template(tpl);
  return fn({
    core: { docker: { commandArgs: [] }, log: { filePath: null } },
    platform: {
      drive: {
        abci: {
          logs: {},
          docker: { build: { buildArgs: buildArgsByService.drive || {} } },
        },
        tenderdash: { log: { path: null } },
      },
      dapi: {
        rsDapi: {
          metrics: { enabled: false },
          docker: { build: { buildArgs: buildArgsByService.rsDapi || {} } },
        },
      },
      gateway: { log: { accessLogs: [] } },
    },
  });
}

describe('dynamic-compose buildArgs rendering', () => {
  it('emits drive_abci build.args from drive.abci.docker.build.buildArgs', () => {
    const out = render({
      drive: { SDK_TEST_DATA: 'true', CARGO_BUILD_PROFILE: 'release' },
    });
    expect(out).to.match(/drive_abci:[\s\S]*build:[\s\S]*args:[\s\S]*SDK_TEST_DATA:\s*"true"/);
    expect(out).to.match(/drive_abci:[\s\S]*CARGO_BUILD_PROFILE:\s*"release"/);
  });

  it('emits rs_dapi build.args from dapi.rsDapi.docker.build.buildArgs', () => {
    const out = render({
      rsDapi: { CARGO_BUILD_PROFILE: 'release' },
    });
    expect(out).to.match(/rs_dapi:[\s\S]*build:[\s\S]*args:[\s\S]*CARGO_BUILD_PROFILE:\s*"release"/);
  });

  it('omits the build block entirely when buildArgs is empty', () => {
    const out = render({});
    // The drive_abci service block is only emitted by this template when
    // either driveLogs or driveBuildArgs has entries — empty buildArgs +
    // empty logs ⇒ no drive_abci section at all.
    expect(out).to.not.match(/drive_abci:\s*\n\s*build:/);
    // rs_dapi is emitted regardless (it carries the expose stanza); but it
    // must NOT carry a build section when buildArgs is empty.
    expect(out).to.not.match(/rs_dapi:[\s\S]*build:\s*\n\s*args:/);
  });
});
