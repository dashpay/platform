import os from 'os';
import path from 'path';
import fs from 'fs';
import { create } from 'tar';
import createSecretScrubber from './createSecretScrubber.js';

function writeSampleFile(archiveDir, service, filename, data, scrub) {
  if (data === undefined || data === null) {
    return;
  }

  const serviceDir = path.join(archiveDir, service ?? '');

  let buffer;
  let filetype;

  const dataType = typeof data;

  if (dataType === 'string') {
    buffer = scrub(data);
    filetype = '.txt';
  } else {
    // Scrubbed value by value rather than as text, so keys, numbers and the
    // structure are left alone and doctor can still read the sample back
    buffer = JSON.stringify(
      data,
      (_key, value) => (typeof value === 'string' ? scrub(value) : value),
      2,
    );
    filetype = '.json';
  }

  if (!fs.existsSync(serviceDir)) {
    fs.mkdirSync(serviceDir);
  }

  fs.writeFileSync(path.join(serviceDir, `${filename}${filetype}`), buffer, 'utf8');
}

/**
 * @param {Samples} samples
 * @param {string} folderPath
 * @param {Config} config - the config the samples were collected from, not obfuscated
 */
export default async function archiveSamples(samples, folderPath, config) {
  if (!config) {
    throw new Error('The config the samples were collected from is needed to remove its secrets');
  }

  const scrub = createSecretScrubber(config);

  const archiveName = `dashmate-report-${samples.date.toISOString()}`;

  // Staged in a fresh directory only the operator can read, and removed once
  // packed: the archive is the one copy meant to leave the machine, and
  // nothing that was in the temp directory before is packed along with it.
  const archiveDir = fs.mkdtempSync(path.join(os.tmpdir(), `${archiveName}-`));

  try {
    // What dashmate writes about itself rather than collects is left out of
    // the scrub. Doctor checks its structure when it reads a report back - the
    // config copy against the config schema, the date as a date - and the
    // scrub cannot tell a secret from an equal value that has to stay, such
    // as a password spelled like the environment name. The config copy has
    // its secrets hidden field by field instead.
    const keep = (text) => text;

    writeSampleFile(archiveDir, null, 'date', samples.date.toISOString(), keep);
    writeSampleFile(archiveDir, null, 'systemInfo', samples.getSystemInfo(), scrub);
    writeSampleFile(archiveDir, null, 'dockerError', samples.getStringifiedDockerError(), scrub);
    writeSampleFile(archiveDir, null, 'dashmateConfig', samples.getDashmateConfig(), keep);
    writeSampleFile(archiveDir, null, 'dashmateVersion', samples.getDashmateVersion(), keep);

    for (const [serviceName, service] of Object.entries(samples.getServices())) {
      for (const [key, data] of Object.entries(service)) {
        if (data !== undefined && data !== null) {
          writeSampleFile(archiveDir, serviceName, key, data, scrub);
        }
      }
    }

    // The staging directory's own entry is left out: it would carry the
    // private mode onto whatever directory the archive is extracted into
    await create(
      {
        cwd: archiveDir,
        gzip: true,
        // Without this every entry records the name and ids of the account
        // that ran the report, which the samples themselves have masked
        portable: true,
        file: path.join(folderPath, `${archiveName}.tar.gz`),
      },
      fs.readdirSync(archiveDir),
    );
  } finally {
    fs.rmSync(archiveDir, { recursive: true, force: true });
  }
}
